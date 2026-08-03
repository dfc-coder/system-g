use crate::{
    IrrigationBlockReason, IrrigationConfig, IrrigationState, ScheduledEventKind, TankState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrrigationEvent {
    LightsTurnedOff { soil_moisture_pct: Option<u8> },
    LightsTurnedOn,
    TankChecked(TankState),
    PumpPulseFinished,
    PumpSafetyTimeout,
    AbsorptionFinished { soil_moisture_pct: Option<u8> },
    Abort(IrrigationBlockReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrrigationAction {
    SetPump(bool),
    ArmPumpSafety {
        after_seconds: u32,
    },
    DisarmPumpSafety,
    MeasureTank,
    Schedule {
        after_seconds: u32,
        kind: ScheduledEventKind,
    },
    CancelScheduled(ScheduledEventKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrrigationDecision {
    pub state: IrrigationState,
    pub actions: Vec<IrrigationAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrrigationMachine {
    state: IrrigationState,
    completed_pulses: u8,
}

impl Default for IrrigationMachine {
    fn default() -> Self {
        Self {
            state: IrrigationState::Idle,
            completed_pulses: 0,
        }
    }
}

impl IrrigationMachine {
    pub fn state(&self) -> IrrigationState {
        self.state
    }

    pub fn completed_pulses(&self) -> u8 {
        self.completed_pulses
    }

    pub fn handle(
        &mut self,
        event: IrrigationEvent,
        config: IrrigationConfig,
    ) -> IrrigationDecision {
        let mut actions = Vec::new();

        self.state = match event {
            IrrigationEvent::LightsTurnedOn => {
                if matches!(self.state, IrrigationState::Pumping { .. }) {
                    actions.push(IrrigationAction::SetPump(false));
                }
                actions.push(IrrigationAction::DisarmPumpSafety);
                actions.push(IrrigationAction::CancelScheduled(
                    ScheduledEventKind::PumpPulseFinished,
                ));
                actions.push(IrrigationAction::CancelScheduled(
                    ScheduledEventKind::AbsorptionFinished,
                ));
                self.completed_pulses = 0;
                IrrigationState::Idle
            }
            IrrigationEvent::Abort(reason) => {
                actions.push(IrrigationAction::SetPump(false));
                actions.push(IrrigationAction::DisarmPumpSafety);
                actions.push(IrrigationAction::CancelScheduled(
                    ScheduledEventKind::PumpPulseFinished,
                ));
                actions.push(IrrigationAction::CancelScheduled(
                    ScheduledEventKind::AbsorptionFinished,
                ));
                IrrigationState::Blocked(reason)
            }
            IrrigationEvent::LightsTurnedOff { soil_moisture_pct } => {
                self.completed_pulses = 0;
                match soil_moisture_pct {
                    None => IrrigationState::Blocked(IrrigationBlockReason::SoilSensorFault),
                    Some(value) if value < config.start_below_pct => {
                        actions.push(IrrigationAction::MeasureTank);
                        IrrigationState::CheckingTank
                    }
                    Some(_) => IrrigationState::Completed,
                }
            }
            IrrigationEvent::TankChecked(tank) => match (self.state, tank) {
                (IrrigationState::CheckingTank, TankState::WaterAvailable)
                    if self.completed_pulses < config.max_pulses_per_cycle =>
                {
                    let pulse = self.completed_pulses + 1;
                    actions.push(IrrigationAction::ArmPumpSafety {
                        after_seconds: u32::from(config.pump_maximum_seconds),
                    });
                    actions.push(IrrigationAction::SetPump(true));
                    actions.push(IrrigationAction::Schedule {
                        after_seconds: u32::from(config.pump_pulse_seconds),
                        kind: ScheduledEventKind::PumpPulseFinished,
                    });
                    IrrigationState::Pumping { pulse }
                }
                (IrrigationState::CheckingTank, TankState::WaterLow) => {
                    IrrigationState::Blocked(IrrigationBlockReason::TankLow)
                }
                (IrrigationState::Absorbing { .. }, TankState::WaterLow) => {
                    actions.push(IrrigationAction::CancelScheduled(
                        ScheduledEventKind::AbsorptionFinished,
                    ));
                    IrrigationState::Blocked(IrrigationBlockReason::TankLow)
                }
                (IrrigationState::CheckingTank, TankState::SensorFault) => {
                    IrrigationState::Blocked(IrrigationBlockReason::TankSensorFault)
                }
                (IrrigationState::Absorbing { .. }, TankState::SensorFault) => {
                    actions.push(IrrigationAction::CancelScheduled(
                        ScheduledEventKind::AbsorptionFinished,
                    ));
                    IrrigationState::Blocked(IrrigationBlockReason::TankSensorFault)
                }
                (IrrigationState::CheckingTank, TankState::WaterAvailable) => {
                    IrrigationState::Blocked(IrrigationBlockReason::PulseLimitReached)
                }
                (state, _) => state,
            },
            IrrigationEvent::PumpPulseFinished => match self.state {
                IrrigationState::Pumping { pulse } => {
                    self.completed_pulses = pulse;
                    actions.push(IrrigationAction::SetPump(false));
                    actions.push(IrrigationAction::DisarmPumpSafety);
                    actions.push(IrrigationAction::Schedule {
                        after_seconds: u32::from(config.absorption_minutes) * 60,
                        kind: ScheduledEventKind::AbsorptionFinished,
                    });
                    actions.push(IrrigationAction::MeasureTank);
                    IrrigationState::Absorbing { pulse }
                }
                state => state,
            },
            IrrigationEvent::PumpSafetyTimeout => match self.state {
                IrrigationState::Pumping { .. } => {
                    actions.push(IrrigationAction::SetPump(false));
                    actions.push(IrrigationAction::DisarmPumpSafety);
                    actions.push(IrrigationAction::CancelScheduled(
                        ScheduledEventKind::PumpPulseFinished,
                    ));
                    IrrigationState::Blocked(IrrigationBlockReason::PumpTimeout)
                }
                state => state,
            },
            IrrigationEvent::AbsorptionFinished { soil_moisture_pct } => match self.state {
                IrrigationState::Absorbing { .. } => match soil_moisture_pct {
                    None => IrrigationState::Blocked(IrrigationBlockReason::SoilSensorFault),
                    Some(value) if value >= config.target_pct => IrrigationState::Completed,
                    Some(_) if self.completed_pulses >= config.max_pulses_per_cycle => {
                        IrrigationState::Blocked(IrrigationBlockReason::PulseLimitReached)
                    }
                    Some(_) => {
                        actions.push(IrrigationAction::MeasureTank);
                        IrrigationState::CheckingTank
                    }
                },
                state => state,
            },
        };

        IrrigationDecision {
            state: self.state,
            actions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> IrrigationConfig {
        IrrigationConfig {
            start_below_pct: 30,
            target_pct: 42,
            pump_pulse_seconds: 8,
            pump_maximum_seconds: 15,
            absorption_minutes: 5,
            max_pulses_per_cycle: 3,
            only_when_lights_off: true,
        }
    }

    fn start_pulse(machine: &mut IrrigationMachine) {
        machine.handle(
            IrrigationEvent::LightsTurnedOff {
                soil_moisture_pct: Some(20),
            },
            config(),
        );
        machine.handle(
            IrrigationEvent::TankChecked(TankState::WaterAvailable),
            config(),
        );
    }

    #[test]
    fn dry_soil_after_lights_off_requests_tank_measurement() {
        let mut machine = IrrigationMachine::default();
        let decision = machine.handle(
            IrrigationEvent::LightsTurnedOff {
                soil_moisture_pct: Some(20),
            },
            config(),
        );

        assert_eq!(decision.state, IrrigationState::CheckingTank);
        assert_eq!(decision.actions, vec![IrrigationAction::MeasureTank]);
    }

    #[test]
    fn tank_low_never_starts_pump() {
        let mut machine = IrrigationMachine::default();
        machine.handle(
            IrrigationEvent::LightsTurnedOff {
                soil_moisture_pct: Some(20),
            },
            config(),
        );
        let decision = machine.handle(IrrigationEvent::TankChecked(TankState::WaterLow), config());

        assert_eq!(
            decision.state,
            IrrigationState::Blocked(IrrigationBlockReason::TankLow)
        );
        assert!(!decision.actions.contains(&IrrigationAction::SetPump(true)));
    }

    #[test]
    fn pump_start_arms_watchdog_before_energizing_output() {
        let mut machine = IrrigationMachine::default();
        machine.handle(
            IrrigationEvent::LightsTurnedOff {
                soil_moisture_pct: Some(20),
            },
            config(),
        );
        let start = machine.handle(
            IrrigationEvent::TankChecked(TankState::WaterAvailable),
            config(),
        );

        assert_eq!(
            start.actions,
            vec![
                IrrigationAction::ArmPumpSafety { after_seconds: 15 },
                IrrigationAction::SetPump(true),
                IrrigationAction::Schedule {
                    after_seconds: 8,
                    kind: ScheduledEventKind::PumpPulseFinished,
                },
            ]
        );
    }

    #[test]
    fn pump_pulse_is_bounded_and_absorption_is_five_minutes() {
        let mut machine = IrrigationMachine::default();
        start_pulse(&mut machine);

        let finish = machine.handle(IrrigationEvent::PumpPulseFinished, config());
        assert!(finish.actions.contains(&IrrigationAction::SetPump(false)));
        assert!(finish.actions.contains(&IrrigationAction::DisarmPumpSafety));
        assert!(finish.actions.contains(&IrrigationAction::MeasureTank));
        assert!(finish.actions.contains(&IrrigationAction::Schedule {
            after_seconds: 300,
            kind: ScheduledEventKind::AbsorptionFinished,
        }));
    }

    #[test]
    fn pump_safety_timeout_stops_and_blocks_cycle() {
        let mut machine = IrrigationMachine::default();
        start_pulse(&mut machine);

        let decision = machine.handle(IrrigationEvent::PumpSafetyTimeout, config());

        assert_eq!(
            decision.state,
            IrrigationState::Blocked(IrrigationBlockReason::PumpTimeout)
        );
        assert!(decision.actions.contains(&IrrigationAction::SetPump(false)));
        assert!(decision
            .actions
            .contains(&IrrigationAction::DisarmPumpSafety));
    }

    #[test]
    fn lights_turning_on_stops_active_pump_and_cancels_pending_work() {
        let mut machine = IrrigationMachine::default();
        start_pulse(&mut machine);

        let decision = machine.handle(IrrigationEvent::LightsTurnedOn, config());
        assert_eq!(decision.state, IrrigationState::Idle);
        assert!(decision.actions.contains(&IrrigationAction::SetPump(false)));
        assert!(decision
            .actions
            .contains(&IrrigationAction::CancelScheduled(
                ScheduledEventKind::PumpPulseFinished
            )));
        assert!(decision
            .actions
            .contains(&IrrigationAction::CancelScheduled(
                ScheduledEventKind::AbsorptionFinished
            )));
    }

    #[test]
    fn post_pulse_tank_low_cancels_absorption_and_blocks_cycle() {
        let mut machine = IrrigationMachine::default();
        start_pulse(&mut machine);
        machine.handle(IrrigationEvent::PumpPulseFinished, config());

        let tank_decision =
            machine.handle(IrrigationEvent::TankChecked(TankState::WaterLow), config());
        assert_eq!(
            tank_decision.state,
            IrrigationState::Blocked(IrrigationBlockReason::TankLow)
        );
        assert!(tank_decision
            .actions
            .contains(&IrrigationAction::CancelScheduled(
                ScheduledEventKind::AbsorptionFinished
            )));
    }

    #[test]
    fn abort_stops_pump_and_leaves_cycle_terminally_blocked() {
        let mut machine = IrrigationMachine::default();
        start_pulse(&mut machine);

        let decision = machine.handle(
            IrrigationEvent::Abort(IrrigationBlockReason::ClockFault),
            config(),
        );

        assert_eq!(
            decision.state,
            IrrigationState::Blocked(IrrigationBlockReason::ClockFault)
        );
        assert!(decision.actions.contains(&IrrigationAction::SetPump(false)));
        assert!(decision
            .actions
            .contains(&IrrigationAction::DisarmPumpSafety));

        let late_pulse = machine.handle(IrrigationEvent::PumpPulseFinished, config());
        assert_eq!(
            late_pulse.state,
            IrrigationState::Blocked(IrrigationBlockReason::ClockFault)
        );
        assert!(late_pulse.actions.is_empty());
    }
}
