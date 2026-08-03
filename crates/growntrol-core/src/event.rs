use crate::{ClimateReading, TankState};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ControlEvent {
    Boot,
    LightTransition { minute_of_day: u16 },
    ClimateMeasured(ClimateReading),
    SoilMeasured { moisture_pct: u8 },
    TankMeasured(TankState),
    PumpPulseFinished,
    AbsorptionFinished,
    ConfigurationChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduledEventKind {
    LightTransition,
    ClimateSample,
    PumpPulseFinished,
    AbsorptionFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledEvent {
    pub due_at_seconds: u64,
    pub kind: ScheduledEventKind,
}
