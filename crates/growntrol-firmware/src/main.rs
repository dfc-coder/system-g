mod adapters;
mod ports;
mod runtime;

use anyhow::Result;
use esp_idf_hal::adc::config::Config as AdcConfig;
use esp_idf_hal::adc::{attenuation, AdcChannelDriver, AdcDriver};
use esp_idf_hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::prelude::*;
use esp_idf_hal::sys::adc_atten_t;
use esp_idf_svc::log::EspLogger;
use growntrol_core::{SoilCalibration, SystemConfig};
use log::info;

use crate::adapters::{ActiveOutput, Dht22, Ds3231, SoilAdc, TankFloat};
use crate::runtime::Runtime;

const RELAYS_ACTIVE_LOW: bool = true;
const PUMP_ACTIVE_LOW: bool = false;
const TANK_AVAILABLE_WHEN_LOW: bool = true;

const SOIL_DRY_RAW: u16 = 3200;
const SOIL_WET_RAW: u16 = 1400;

fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();
    info!("Growntrol firmware starting");

    let peripherals = Peripherals::take()?;

    // Outputs are constructed first and immediately driven to their inactive level.
    let mut lights = ActiveOutput::new(peripherals.pins.gpio25, RELAYS_ACTIVE_LOW)?;
    let mut fans = ActiveOutput::new(peripherals.pins.gpio26, RELAYS_ACTIVE_LOW)?;
    let mut pump = ActiveOutput::new(peripherals.pins.gpio27, PUMP_ACTIVE_LOW)?;

    let mut climate = Dht22::new(peripherals.pins.gpio4)?;
    let mut tank = TankFloat::new(peripherals.pins.gpio32, TANK_AVAILABLE_WHEN_LOW)?;

    const SOIL_ATTENUATION: adc_atten_t = attenuation::DB_12;
    let adc = AdcDriver::new(peripherals.adc1, &AdcConfig::new().calibration(true))?;
    let adc_channel: AdcChannelDriver<{ SOIL_ATTENUATION }, _> =
        AdcChannelDriver::new(peripherals.pins.gpio34)?;
    let mut soil = SoilAdc::new(
        adc,
        adc_channel,
        SoilCalibration {
            dry_raw: SOIL_DRY_RAW,
            wet_raw: SOIL_WET_RAW,
        },
    )?;

    let i2c_config = I2cConfig::new().baudrate(100.kHz().into());
    let i2c = I2cDriver::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        &i2c_config,
    )?;
    let mut clock = Ds3231::new(i2c);

    let config = SystemConfig::default();
    let mut runtime = Runtime::new(
        config,
        &mut lights,
        &mut fans,
        &mut pump,
        &mut climate,
        &mut soil,
        &mut tank,
        &mut clock,
    )?;

    runtime.run_forever()
}
