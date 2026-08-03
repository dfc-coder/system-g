use anyhow::Result;
use growntrol_core::{ClimateReading, TankState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RtcDateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl RtcDateTime {
    pub fn minute_of_day(self) -> u16 {
        u16::from(self.hour) * 60 + u16::from(self.minute)
    }

    pub fn validate(self) -> Result<Self> {
        anyhow::ensure!((1..=12).contains(&self.month), "invalid RTC month");
        anyhow::ensure!((1..=31).contains(&self.day), "invalid RTC day");
        anyhow::ensure!(self.hour < 24, "invalid RTC hour");
        anyhow::ensure!(self.minute < 60, "invalid RTC minute");
        anyhow::ensure!(self.second < 60, "invalid RTC second");
        Ok(self)
    }
}

pub trait Actuator {
    fn set(&mut self, on: bool) -> Result<bool>;
    fn is_on(&self) -> bool;
}

pub trait ClimateSensor {
    fn read(&mut self) -> Result<ClimateReading>;
}

pub trait SoilSensor {
    fn read_percent(&mut self) -> Result<u8>;
}

pub trait TankSensor {
    fn read(&mut self) -> Result<TankState>;
}

pub trait WallClock {
    fn now(&mut self) -> Result<RtcDateTime>;
}
