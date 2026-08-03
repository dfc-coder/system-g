use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LightingConfig {
    pub start_minute: u16,
    pub duration_minutes: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FanConfig {
    pub temperature_on_c: f32,
    pub temperature_off_c: f32,
    pub humidity_on_pct: f32,
    pub humidity_off_pct: f32,
    pub normal_sample_minutes: u16,
    pub alert_sample_minutes: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IrrigationConfig {
    pub start_below_pct: u8,
    pub target_pct: u8,
    pub pump_pulse_seconds: u16,
    pub pump_maximum_seconds: u16,
    pub absorption_minutes: u16,
    pub max_pulses_per_cycle: u8,
    pub only_when_lights_off: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SystemConfig {
    pub lighting: LightingConfig,
    pub fan: FanConfig,
    pub irrigation: IrrigationConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            lighting: LightingConfig {
                start_minute: 20 * 60,
                duration_minutes: 18 * 60,
            },
            fan: FanConfig {
                temperature_on_c: 29.0,
                temperature_off_c: 27.0,
                humidity_on_pct: 75.0,
                humidity_off_pct: 68.0,
                normal_sample_minutes: 5,
                alert_sample_minutes: 1,
            },
            irrigation: IrrigationConfig {
                start_below_pct: 30,
                target_pct: 42,
                pump_pulse_seconds: 8,
                pump_maximum_seconds: 15,
                absorption_minutes: 5,
                max_pulses_per_cycle: 3,
                only_when_lights_off: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    InvalidLightingDuration,
    InvalidFanTemperatureHysteresis,
    InvalidFanHumidityHysteresis,
    InvalidSamplingInterval,
    InvalidSoilThresholds,
    InvalidPumpPulse,
    InvalidPumpMaximum,
    InvalidAbsorptionTime,
    InvalidPulseLimit,
}

impl SystemConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.lighting.duration_minutes == 0 || self.lighting.duration_minutes > 24 * 60 {
            return Err(ConfigError::InvalidLightingDuration);
        }
        if self.fan.temperature_off_c >= self.fan.temperature_on_c {
            return Err(ConfigError::InvalidFanTemperatureHysteresis);
        }
        if self.fan.humidity_off_pct >= self.fan.humidity_on_pct {
            return Err(ConfigError::InvalidFanHumidityHysteresis);
        }
        if self.fan.normal_sample_minutes == 0 || self.fan.alert_sample_minutes == 0 {
            return Err(ConfigError::InvalidSamplingInterval);
        }
        if self.irrigation.start_below_pct >= self.irrigation.target_pct
            || self.irrigation.target_pct > 100
        {
            return Err(ConfigError::InvalidSoilThresholds);
        }
        if self.irrigation.pump_pulse_seconds == 0 {
            return Err(ConfigError::InvalidPumpPulse);
        }
        if self.irrigation.pump_maximum_seconds < self.irrigation.pump_pulse_seconds {
            return Err(ConfigError::InvalidPumpMaximum);
        }
        if self.irrigation.absorption_minutes == 0 {
            return Err(ConfigError::InvalidAbsorptionTime);
        }
        if self.irrigation.max_pulses_per_cycle == 0 {
            return Err(ConfigError::InvalidPulseLimit);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configuration_is_valid() {
        assert_eq!(SystemConfig::default().validate(), Ok(()));
    }

    #[test]
    fn rejects_missing_fan_hysteresis() {
        let mut config = SystemConfig::default();
        config.fan.temperature_off_c = config.fan.temperature_on_c;
        assert_eq!(
            config.validate(),
            Err(ConfigError::InvalidFanTemperatureHysteresis)
        );
    }

    #[test]
    fn rejects_pump_maximum_shorter_than_normal_pulse() {
        let mut config = SystemConfig::default();
        config.irrigation.pump_maximum_seconds = config.irrigation.pump_pulse_seconds - 1;
        assert_eq!(config.validate(), Err(ConfigError::InvalidPumpMaximum));
    }
}
