mod adapters;
mod ports;
mod runtime;

use std::sync::mpsc;

use anyhow::Result;
use esp_idf_hal::gpio::Pins;
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

    // The firmware owns these pins for its entire lifetime. Constructing GPIOs directly avoids
    // pulling the legacy HAL I2C/ADC modules into the final image through `Peripherals::take()`.
    let pins = unsafe { Pins::new() };
    let (hardware_event_tx, hardware_event_rx) = mpsc::channel();

    // Outputs are constructed first and immediately driven to their inactive level.
    let mut lights = ActiveOutput::new(pins.gpio25, RELAYS_ACTIVE_LOW)?;
    let mut fans = ActiveOutput::new(pins.gpio26, RELAYS_ACTIVE_LOW)?;
    let mut pump = PumpOutput::new(pins.gpio27, PUMP_ACTIVE_LOW, hardware_event_tx)?;

    let mut climate = Dht22::new(pins.gpio4)?;
    let mut tank = TankFloat::new(pins.gpio32, TANK_AVAILABLE_WHEN_LOW)?;

    let mut soil = SoilAdc::new(
        pins.gpio34,
        esp_idf_svc::sys::adc_atten_t_ADC_ATTEN_DB_12,
        SoilCalibration {
            dry_raw: SOIL_DRY_RAW,
            wet_raw: SOIL_WET_RAW,
        },
    )?;

    let mut clock = Ds3231::new(pins.gpio21, pins.gpio22)?;

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
