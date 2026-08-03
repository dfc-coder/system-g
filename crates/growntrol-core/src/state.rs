use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FanState {
    Off,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TankState {
    WaterAvailable,
    WaterLow,
    SensorFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrrigationState {
    Idle,
    CheckingSoil,
    CheckingTank,
    Pumping { pulse: u8 },
    Absorbing { pulse: u8 },
    Completed,
    Blocked(IrrigationBlockReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IrrigationBlockReason {
    TankLow,
    TankSensorFault,
    SoilSensorFault,
    LightsOn,
    PulseLimitReached,
    PumpTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClimateReading {
    pub temperature_c: f32,
    pub humidity_pct: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputState {
    pub lights_on: bool,
    pub fans: FanState,
    pub pump_on: bool,
}

impl Default for OutputState {
    fn default() -> Self {
        Self {
            lights_on: false,
            fans: FanState::Off,
            pump_on: false,
        }
    }
}
