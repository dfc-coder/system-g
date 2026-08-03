mod adapters;
mod ports;
mod runtime;

use std::sync::mpsc;

use anyhow::Result;
use esp_idf_hal::adc::attenuation;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::log::EspLogger;
use growntrol_core::{SoilCalibration, SystemConfig};
use log::info;

use crate::adapters::{ActiveOutput, Dht22, Ds3231, PumpOutput, SoilAdc, TankFloat};
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
    let (hardware_event_tx, hardware_event_rx) = mpsc::channel();

    // Outputs are constructed first and immediately driven to their inactive level.
    let mut lights = ActiveOutput::new(peripherals.pins.gpio25, RELAYS_ACTIVE_LOW)?;
    let mut fans = ActiveOutput::new(peripherals.pins.gpio26, RELAYS_ACTIVE_LOW)?;
    let mut pump = PumpOutput::new(peripherals.pins.gpio27, PUMP_ACTIVE_LOW, hardware_event_tx)?;

    let mut climate = Dht22::new(peripherals.pins.gpio4)?;
    let mut tank = TankFloat::new(peripherals.pins.gpio32, TANK_AVAILABLE_WHEN_LOW)?;

    let mut soil = SoilAdc::new(
        peripherals.adc1,
        peripherals.pins.gpio34,
        attenuation::DB_12,
        SoilCalibration {
            dry_raw: SOIL_DRY_RAW,
            wet_raw: SOIL_WET_RAW,
        },
    )?;

    let mut clock = Ds3231::new(
        peripherals.i2c0,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    )?;

    let config = SystemConfig::default();
    let mut runtime = Runtime::new(
        config,
        hardware_event_rx,
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
