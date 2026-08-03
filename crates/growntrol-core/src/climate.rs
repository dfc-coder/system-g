use crate::{ClimateReading, FanConfig, FanState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FanDecision {
    pub desired_state: FanState,
    pub write_relay: bool,
    pub next_sample_minutes: u16,
}

pub fn evaluate_fan(current: FanState, reading: ClimateReading, config: FanConfig) -> FanDecision {
    let desired_state = match current {
        FanState::Off
            if reading.temperature_c >= config.temperature_on_c
                || reading.humidity_pct >= config.humidity_on_pct =>
        {
            FanState::On
        }
        FanState::On
            if reading.temperature_c <= config.temperature_off_c
                && reading.humidity_pct <= config.humidity_off_pct =>
        {
            FanState::Off
        }
        _ => current,
    };

    let near_threshold = reading.temperature_c >= config.temperature_off_c
        || reading.humidity_pct >= config.humidity_off_pct;
    let next_sample_minutes = if desired_state == FanState::On || near_threshold {
        config.alert_sample_minutes
    } else {
        config.normal_sample_minutes
    };

    FanDecision {
        desired_state,
        write_relay: desired_state != current,
        next_sample_minutes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> FanConfig {
        FanConfig {
            temperature_on_c: 29.0,
            temperature_off_c: 27.0,
            humidity_on_pct: 75.0,
            humidity_off_pct: 68.0,
            normal_sample_minutes: 5,
            alert_sample_minutes: 1,
        }
    }

    #[test]
    fn repeated_high_readings_do_not_repeat_relay_writes() {
        let first = evaluate_fan(
            FanState::Off,
            ClimateReading {
                temperature_c: 29.1,
                humidity_pct: 60.0,
            },
            config(),
        );
        assert_eq!(first.desired_state, FanState::On);
        assert!(first.write_relay);

        let second = evaluate_fan(
            FanState::On,
            ClimateReading {
                temperature_c: 30.0,
                humidity_pct: 60.0,
            },
            config(),
        );
        assert_eq!(second.desired_state, FanState::On);
        assert!(!second.write_relay);
    }

    #[test]
    fn hysteresis_preserves_state_between_thresholds() {
        let decision = evaluate_fan(
            FanState::On,
            ClimateReading {
                temperature_c: 28.0,
                humidity_pct: 60.0,
            },
            config(),
        );
        assert_eq!(decision.desired_state, FanState::On);
        assert!(!decision.write_relay);
    }

    #[test]
    fn uses_one_minute_sampling_near_threshold_or_while_on() {
        let near = evaluate_fan(
            FanState::Off,
            ClimateReading {
                temperature_c: 27.5,
                humidity_pct: 60.0,
            },
            config(),
        );
        assert_eq!(near.next_sample_minutes, 1);

        let stable = evaluate_fan(
            FanState::Off,
            ClimateReading {
                temperature_c: 24.0,
                humidity_pct: 55.0,
            },
            config(),
        );
        assert_eq!(stable.next_sample_minutes, 5);
    }
}
