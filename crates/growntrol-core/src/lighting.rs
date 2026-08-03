use crate::LightingConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightingDecision {
    pub lights_on: bool,
    pub next_transition_minute: u16,
}

pub fn evaluate_lighting(minute_of_day: u16, config: LightingConfig) -> LightingDecision {
    let now = minute_of_day % 1440;
    let start = config.start_minute % 1440;
    let duration = config.duration_minutes.min(1440);
    let elapsed = (now + 1440 - start) % 1440;
    let lights_on = elapsed < duration;
    let end = (start + duration) % 1440;
    let next_transition_minute = if lights_on { end } else { start };

    LightingDecision {
        lights_on,
        next_transition_minute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schedule() -> LightingConfig {
        LightingConfig {
            start_minute: 20 * 60,
            duration_minutes: 18 * 60,
        }
    }

    #[test]
    fn cycle_crosses_midnight() {
        assert!(evaluate_lighting(23 * 60, schedule()).lights_on);
        assert!(evaluate_lighting(8 * 60, schedule()).lights_on);
        assert!(!evaluate_lighting(15 * 60, schedule()).lights_on);
    }

    #[test]
    fn turns_off_exactly_at_cycle_end() {
        assert!(evaluate_lighting(13 * 60 + 59, schedule()).lights_on);
        assert!(!evaluate_lighting(14 * 60, schedule()).lights_on);
    }

    #[test]
    fn schedules_only_the_next_transition() {
        let decision = evaluate_lighting(10 * 60 + 30, schedule());
        assert!(decision.lights_on);
        assert_eq!(decision.next_transition_minute, 14 * 60);
    }
}
