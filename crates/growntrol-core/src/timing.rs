pub const SECONDS_PER_DAY: u32 = 24 * 60 * 60;

pub fn seconds_until_minute(
    hour: u8,
    minute: u8,
    second: u8,
    target_minute_of_day: u16,
) -> u32 {
    let now = u32::from(hour % 24) * 3600
        + u32::from(minute % 60) * 60
        + u32::from(second % 60);
    let target = u32::from(target_minute_of_day % 1440) * 60;
    let delta = (target + SECONDS_PER_DAY - now) % SECONDS_PER_DAY;

    if delta == 0 {
        SECONDS_PER_DAY
    } else {
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_transition_later_the_same_day() {
        assert_eq!(seconds_until_minute(10, 30, 0, 14 * 60), 3 * 3600 + 30 * 60);
    }

    #[test]
    fn calculates_transition_across_midnight() {
        assert_eq!(seconds_until_minute(23, 0, 0, 2 * 60), 3 * 3600);
    }

    #[test]
    fn exact_transition_schedules_the_next_day_not_immediate_repetition() {
        assert_eq!(seconds_until_minute(14, 0, 0, 14 * 60), SECONDS_PER_DAY);
    }
}
