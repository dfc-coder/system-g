use crate::{ScheduledEvent, ScheduledEventKind};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DeadlineScheduler {
    events: Vec<ScheduledEvent>,
}

impl DeadlineScheduler {
    pub fn schedule_after(
        &mut self,
        now_seconds: u64,
        after_seconds: u32,
        kind: ScheduledEventKind,
    ) {
        self.cancel(kind);
        self.events.push(ScheduledEvent {
            due_at_seconds: now_seconds.saturating_add(u64::from(after_seconds)),
            kind,
        });
        self.events.sort_by_key(|event| event.due_at_seconds);
    }

    pub fn cancel(&mut self, kind: ScheduledEventKind) {
        self.events.retain(|event| event.kind != kind);
    }

    pub fn take_due(&mut self, now_seconds: u64) -> Vec<ScheduledEventKind> {
        let split = self
            .events
            .partition_point(|event| event.due_at_seconds <= now_seconds);
        self.events.drain(..split).map(|event| event.kind).collect()
    }

    pub fn next_due_at(&self) -> Option<u64> {
        self.events.first().map(|event| event.due_at_seconds)
    }

    pub fn contains(&self, kind: ScheduledEventKind) -> bool {
        self.events.iter().any(|event| event.kind == kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_a_deadline_prevents_duplicate_events() {
        let mut scheduler = DeadlineScheduler::default();
        scheduler.schedule_after(100, 10, ScheduledEventKind::ClimateSample);
        scheduler.schedule_after(100, 20, ScheduledEventKind::ClimateSample);

        assert!(scheduler.take_due(115).is_empty());
        assert_eq!(
            scheduler.take_due(120),
            vec![ScheduledEventKind::ClimateSample]
        );
    }

    #[test]
    fn cancel_removes_a_pending_safety_timeout() {
        let mut scheduler = DeadlineScheduler::default();
        scheduler.schedule_after(100, 15, ScheduledEventKind::PumpSafetyTimeout);
        scheduler.cancel(ScheduledEventKind::PumpSafetyTimeout);

        assert!(!scheduler.contains(ScheduledEventKind::PumpSafetyTimeout));
        assert!(scheduler.take_due(200).is_empty());
    }

    #[test]
    fn due_events_are_returned_in_deadline_order() {
        let mut scheduler = DeadlineScheduler::default();
        scheduler.schedule_after(100, 20, ScheduledEventKind::AbsorptionFinished);
        scheduler.schedule_after(100, 5, ScheduledEventKind::PumpPulseFinished);

        assert_eq!(
            scheduler.take_due(120),
            vec![
                ScheduledEventKind::PumpPulseFinished,
                ScheduledEventKind::AbsorptionFinished
            ]
        );
    }
}
