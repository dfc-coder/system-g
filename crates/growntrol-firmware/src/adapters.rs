use core::marker::PhantomData;
use core::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use esp_idf_hal::adc::{Adc, AdcChannel};
use esp_idf_hal::delay::{Ets, FreeRtos};
use esp_idf_hal::gpio::{ADCPin, Input, InputOutput, InputPin, Output, OutputPin, PinDriver, Pull};
use esp_idf_hal::i2c::I2c;
use esp_idf_hal::sys::{self, EspError};
use esp_idf_svc::timer::{EspTaskTimerService, EspTimer};
use growntrol_core::{median_sample, ClimateReading, SoilCalibration, TankState};
use log::error;

use crate::ports::{
    Actuator, ClimateSensor, HardwareEvent, PumpActuator, RtcDateTime, SoilSensor, TankSensor,
    WallClock,
};

fn check_esp(code: sys::esp_err_t, context: &'static str) -> Result<()> {
    EspError::convert(code).context(context)
}

pub struct ActiveOutput<'d> {
    pin: PinDriver<'d, Output>,
    active_low: bool,
    on: Arc<AtomicBool>,
}

impl<'d> ActiveOutput<'d> {
    pub fn new<T: OutputPin + 'd>(pin: T, active_low: bool) -> Result<Self> {
        let pin_number = pin.pin();
        let inactive_level = if active_low { 1 } else { 0 };
        let result = unsafe { sys::gpio_set_level(pin_number as _, inactive_level) };
        anyhow::ensure!(
            result == sys::ESP_OK,
            "failed to preload inactive level for GPIO {pin_number}: {result}"
        );

        let mut pin = PinDriver::output(pin).context("configure output GPIO")?;
        if active_low {
            pin.set_high().context("set safe inactive output")?;
        } else {
            pin.set_low().context("set safe inactive output")?;
        }

        Ok(Self {
            pin,
            active_low,
            on: Arc::new(AtomicBool::new(false)),
        })
    }

    fn raw_pin(&self) -> u8 {
        self.pin.pin()
    }

    fn inactive_level(&self) -> u32 {
        if self.active_low {
            1
        } else {
            0
        }
    }

    fn state_handle(&self) -> Arc<AtomicBool> {
        self.on.clone()
    }
}

impl Actuator for ActiveOutput<'_> {
    fn set(&mut self, on: bool) -> Result<bool> {
        if self.on.load(Ordering::Acquire) == on {
            return Ok(false);
        }

        let drive_high = on != self.active_low;
        if drive_high {
            self.pin.set_high().context("drive output high")?;
        } else {
            self.pin.set_low().context("drive output low")?;
        }
        self.on.store(on, Ordering::Release);
        Ok(true)
    }

    fn is_on(&self) -> bool {
        self.on.load(Ordering::Acquire)
    }
}

pub struct PumpOutput<'d> {
    output: ActiveOutput<'d>,
    safety_timer: EspTimer<'static>,
}

impl<'d> PumpOutput<'d> {
    pub fn new<T: OutputPin + 'd>(
        pin: T,
        active_low: bool,
        event_sender: Sender<HardwareEvent>,
    ) -> Result<Self> {
        let output = ActiveOutput::new(pin, active_low)?;
        let pin_number = output.raw_pin();
        let inactive_level = output.inactive_level();
        let state = output.state_handle();

        let timer_service = EspTaskTimerService::new().context("create ESP timer service")?;
        let safety_timer = timer_service
            .timer(move || {
                if !state.load(Ordering::Acquire) {
                    return;
                }

                let result = unsafe { sys::gpio_set_level(pin_number as _, inactive_level) };
                if result == sys::ESP_OK {
                    state.store(false, Ordering::Release);
                } else {
                    error!(
                        "pump watchdog failed to drive GPIO {} inactive: {}",
                        pin_number, result
                    );
                }

                if event_sender.send(HardwareEvent::PumpSafetyTimeout).is_err() {
                    error!("pump watchdog could not notify control runtime");
                }
            })
            .context("create pump safety timer")?;

        Ok(Self {
            output,
            safety_timer,
        })
    }
}

impl Actuator for PumpOutput<'_> {
    fn set(&mut self, on: bool) -> Result<bool> {
        self.output.set(on)
    }

    fn is_on(&self) -> bool {
        self.output.is_on()
    }
}

impl PumpActuator for PumpOutput<'_> {
    fn arm_safety_timeout(&mut self, duration: Duration) -> Result<()> {
        self.safety_timer
            .after(duration)
            .context("arm pump safety timer")
    }

    fn disarm_safety_timeout(&mut self) -> Result<()> {
        self.safety_timer
            .cancel()
            .context("disarm pump safety timer")?;
        Ok(())
    }
}

pub struct Dht22<'d> {
    pin: PinDriver<'d, InputOutput>,
}

impl<'d> Dht22<'d> {
    pub fn new<T: InputPin + OutputPin + 'd>(pin: T) -> Result<Self> {
        let mut pin = PinDriver::input_output_od(pin, Pull::Up).context("configure DHT22 GPIO")?;
        pin.set_high().context("release DHT22 data line")?;
        Ok(Self { pin })
    }
}

impl ClimateSensor for Dht22<'_> {
    fn read(&mut self) -> Result<ClimateReading> {
        let mut delay = Ets;
        let reading = dht_sensor::dht22::blocking::read(&mut delay, &mut self.pin)
            .map_err(|error| anyhow!("DHT22 read failed: {error:?}"))?;

        anyhow::ensure!(reading.temperature.is_finite(), "invalid DHT22 temperature");
        anyhow::ensure!(
            reading.relative_humidity.is_finite(),
            "invalid DHT22 humidity"
        );
        anyhow::ensure!(
            (0.0..=100.0).contains(&reading.relative_humidity),
            "DHT22 humidity outside range"
        );

        Ok(ClimateReading {
            temperature_c: reading.temperature,
            humidity_pct: reading.relative_humidity,
        })
    }
}

pub struct TankFloat<'d> {
    pin: PinDriver<'d, Input>,
    available_when_low: bool,
}

impl<'d> TankFloat<'d> {
    pub fn new<T: InputPin + 'd>(pin: T, available_when_low: bool) -> Result<Self> {
        Ok(Self {
            pin: PinDriver::input(pin, Pull::Up).context("configure tank float GPIO")?,
            available_when_low,
        })
    }
}

impl TankSensor for TankFloat<'_> {
    fn read(&mut self) -> Result<TankState> {
        let low = self.pin.is_low();
        let available = if self.available_when_low { low } else { !low };
        Ok(if available {
            TankState::WaterAvailable
        } else {
            TankState::WaterLow
        })
    }
}

pub struct SoilAdc<'d> {
    unit: sys::adc_oneshot_unit_handle_t,
    channel: sys::adc_channel_t,
    calibration: SoilCalibration,
    _ownership: PhantomData<&'d mut ()>,
}

impl<'d> SoilAdc<'d> {
    pub fn new<ADC, PIN>(
        _adc: ADC,
        _pin: PIN,
        attenuation: sys::adc_atten_t,
        calibration: SoilCalibration,
    ) -> Result<Self>
    where
        ADC: Adc + 'd,
        PIN: ADCPin + 'd,
        PIN::AdcChannel: AdcChannel<AdcUnit = ADC::AdcUnit>,
    {
        calibration
            .validate()
            .map_err(|error| anyhow!("{error:?}"))?;

        let mut unit = ptr::null_mut();
        let mut unit_config = sys::adc_oneshot_unit_init_cfg_t::default();
        unit_config.unit_id = ADC::unit();
        check_esp(
            unsafe { sys::adc_oneshot_new_unit(&unit_config, &mut unit) },
            "initialize ADC oneshot unit",
        )?;

        let channel = <PIN::AdcChannel as AdcChannel>::channel();
        let channel_config = sys::adc_oneshot_chan_cfg_t {
            atten: attenuation,
            bitwidth: sys::adc_bitwidth_t_ADC_BITWIDTH_DEFAULT,
        };
        if let Err(error) = check_esp(
            unsafe { sys::adc_oneshot_config_channel(unit, channel, &channel_config) },
            "configure soil ADC oneshot channel",
        ) {
            let _ = EspError::convert(unsafe { sys::adc_oneshot_del_unit(unit) });
            return Err(error);
        }

        Ok(Self {
            unit,
            channel,
            calibration,
            _ownership: PhantomData,
        })
    }
}

impl SoilSensor for SoilAdc<'_> {
    fn read_percent(&mut self) -> Result<u8> {
        let mut samples = [0_u16; 7];
        for sample in &mut samples {
            let mut raw = 0_i32;
            check_esp(
                unsafe { sys::adc_oneshot_read(self.unit, self.channel, &mut raw) },
                "read raw soil ADC",
            )?;
            *sample = u16::try_from(raw).context("soil ADC returned a negative value")?;
            FreeRtos::delay_ms(25);
        }

        let raw = median_sample(&mut samples).map_err(|error| anyhow!("{error:?}"))?;
        self.calibration
            .percentage(raw)
            .map_err(|error| anyhow!("{error:?}"))
    }
}

impl Drop for SoilAdc<'_> {
    fn drop(&mut self) {
        if let Some(error) = EspError::from(unsafe { sys::adc_oneshot_del_unit(self.unit) }) {
            error!("failed to release ADC oneshot unit: {error}");
        }
    }
}

pub struct Ds3231<'d> {
    bus: sys::i2c_master_bus_handle_t,
    device: sys::i2c_master_dev_handle_t,
    _ownership: PhantomData<&'d mut ()>,
}

impl<'d> Ds3231<'d> {
    const ADDRESS: u16 = 0x68;
    const I2C_TIMEOUT_MS: i32 = 100;

    pub fn new<I2C, SDA, SCL>(_i2c: I2C, sda: SDA, scl: SCL) -> Result<Self>
    where
        I2C: I2c + 'd,
        SDA: InputPin + OutputPin + 'd,
        SCL: InputPin + OutputPin + 'd,
    {
        let mut bus_config = sys::i2c_master_bus_config_t::default();
        bus_config.i2c_port = I2C::port() as _;
        bus_config.sda_io_num = sda.pin() as _;
        bus_config.scl_io_num = scl.pin() as _;
        bus_config.glitch_ignore_cnt = 7;
        bus_config.flags.set_enable_internal_pullup(1);

        let mut bus = ptr::null_mut();
        check_esp(
            unsafe { sys::i2c_new_master_bus(&bus_config, &mut bus) },
            "initialize I2C master bus",
        )?;

        let mut device_config = sys::i2c_device_config_t::default();
        device_config.dev_addr_length = sys::i2c_addr_bit_len_t_I2C_ADDR_BIT_LEN_7;
        device_config.device_address = Self::ADDRESS;
        device_config.scl_speed_hz = 100_000;

        let mut device = ptr::null_mut();
        if let Err(error) = check_esp(
            unsafe { sys::i2c_master_bus_add_device(bus, &device_config, &mut device) },
            "add DS3231 to I2C bus",
        ) {
            let _ = EspError::convert(unsafe { sys::i2c_del_master_bus(bus) });
            return Err(error);
        }

        Ok(Self {
            bus,
            device,
            _ownership: PhantomData,
        })
    }

    fn write_read(&mut self, register: u8, data: &mut [u8]) -> Result<()> {
        check_esp(
            unsafe {
                sys::i2c_master_transmit_receive(
                    self.device,
                    &register,
                    1,
                    data.as_mut_ptr(),
                    data.len(),
                    Self::I2C_TIMEOUT_MS,
                )
            },
            "I2C transmit-receive",
        )
    }

    fn bcd(value: u8) -> u8 {
        (value >> 4) * 10 + (value & 0x0f)
    }

    fn decode_hour(value: u8) -> u8 {
        if value & 0x40 == 0 {
            return Self::bcd(value & 0x3f);
        }

        let hour_12 = Self::bcd(value & 0x1f);
        let pm = value & 0x20 != 0;
        match (hour_12, pm) {
            (12, false) => 0,
            (12, true) => 12,
            (hour, false) => hour,
            (hour, true) => hour + 12,
        }
    }
}

impl WallClock for Ds3231<'_> {
    fn now(&mut self) -> Result<RtcDateTime> {
        let mut status = [0_u8; 1];
        self.write_read(0x0f, &mut status)
            .context("read DS3231 status")?;
        anyhow::ensure!(status[0] & 0x80 == 0, "DS3231 oscillator-stop flag is set");

        let mut data = [0_u8; 7];
        self.write_read(0x00, &mut data)
            .context("read DS3231 time")?;

        RtcDateTime {
            year: 2000 + u16::from(Self::bcd(data[6])),
            month: Self::bcd(data[5] & 0x1f),
            day: Self::bcd(data[4] & 0x3f),
            hour: Self::decode_hour(data[2]),
            minute: Self::bcd(data[1] & 0x7f),
            second: Self::bcd(data[0] & 0x7f),
        }
        .validate()
    }
}

impl Drop for Ds3231<'_> {
    fn drop(&mut self) {
        if let Some(error) = EspError::from(unsafe { sys::i2c_master_bus_rm_device(self.device) }) {
            error!("failed to remove DS3231 from I2C bus: {error}");
        }
        if let Some(error) = EspError::from(unsafe { sys::i2c_del_master_bus(self.bus) }) {
            error!("failed to release I2C master bus: {error}");
        }
    }
}
