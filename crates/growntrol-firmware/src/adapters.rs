use anyhow::{anyhow, Context, Result};
use embedded_hal::i2c::I2c;
use esp_idf_hal::adc::{ADCPin, AdcChannelDriver, AdcDriver, AdcUnit};
use esp_idf_hal::delay::{Ets, FreeRtos};
use esp_idf_hal::gpio::{Input, InputOutput, InputPin, Output, OutputPin, PinDriver, Pull};
use esp_idf_hal::i2c::I2cDriver;
use esp_idf_hal::sys::adc_atten_t;
use growntrol_core::{median_sample, ClimateReading, SoilCalibration, TankState};

use crate::ports::{Actuator, ClimateSensor, RtcDateTime, SoilSensor, TankSensor, WallClock};

pub struct ActiveOutput<'d> {
    pin: PinDriver<'d, Output>,
    active_low: bool,
    on: bool,
}

impl<'d> ActiveOutput<'d> {
    pub fn new<T: OutputPin + 'd>(pin: T, active_low: bool) -> Result<Self> {
        let mut pin = PinDriver::output(pin).context("configure output GPIO")?;
        if active_low {
            pin.set_high().context("set safe inactive output")?;
        } else {
            pin.set_low().context("set safe inactive output")?;
        }

        Ok(Self {
            pin,
            active_low,
            on: false,
        })
    }
}

impl Actuator for ActiveOutput<'_> {
    fn set(&mut self, on: bool) -> Result<bool> {
        if self.on == on {
            return Ok(false);
        }

        let drive_high = on != self.active_low;
        if drive_high {
            self.pin.set_high().context("drive output high")?;
        } else {
            self.pin.set_low().context("drive output low")?;
        }
        self.on = on;
        Ok(true)
    }

    fn is_on(&self) -> bool {
        self.on
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

pub struct SoilAdc<'d, ADC, PIN, const ATTENUATION: adc_atten_t>
where
    ADC: AdcUnit,
    PIN: ADCPin<Adc = ADC>,
{
    adc: AdcDriver<'d, ADC>,
    channel: AdcChannelDriver<'d, ATTENUATION, PIN>,
    calibration: SoilCalibration,
}

impl<'d, ADC, PIN, const ATTENUATION: adc_atten_t> SoilAdc<'d, ADC, PIN, ATTENUATION>
where
    ADC: AdcUnit,
    PIN: ADCPin<Adc = ADC>,
{
    pub fn new(
        adc: AdcDriver<'d, ADC>,
        channel: AdcChannelDriver<'d, ATTENUATION, PIN>,
        calibration: SoilCalibration,
    ) -> Result<Self> {
        calibration
            .validate()
            .map_err(|error| anyhow!("{error:?}"))?;
        Ok(Self {
            adc,
            channel,
            calibration,
        })
    }
}

impl<ADC, PIN, const ATTENUATION: adc_atten_t> SoilSensor for SoilAdc<'_, ADC, PIN, ATTENUATION>
where
    ADC: AdcUnit,
    PIN: ADCPin<Adc = ADC>,
{
    fn read_percent(&mut self) -> Result<u8> {
        let mut samples = [0_u16; 7];
        for sample in &mut samples {
            *sample = self.adc.read(&mut self.channel).context("read soil ADC")?;
            FreeRtos::delay_ms(25);
        }

        let raw = median_sample(&mut samples).map_err(|error| anyhow!("{error:?}"))?;
        self.calibration
            .percentage(raw)
            .map_err(|error| anyhow!("{error:?}"))
    }
}

pub struct Ds3231<'d> {
    i2c: I2cDriver<'d>,
}

impl<'d> Ds3231<'d> {
    const ADDRESS: u8 = 0x68;

    pub fn new(i2c: I2cDriver<'d>) -> Self {
        Self { i2c }
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
        self.i2c
            .write_read(Self::ADDRESS, &[0x0f], &mut status)
            .context("read DS3231 status")?;
        anyhow::ensure!(status[0] & 0x80 == 0, "DS3231 oscillator-stop flag is set");

        let mut data = [0_u8; 7];
        self.i2c
            .write_read(Self::ADDRESS, &[0x00], &mut data)
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
