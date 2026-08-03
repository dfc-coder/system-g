use crate::{
    IrrigationBlockReason, IrrigationConfig, IrrigationState, ScheduledEventKind, TankState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrrigationEvent {
    LightsTurnedOff { soil_moisture_pct: Option<u8> },
    LightsTurnedOn,
    TankChecked(TankState),
    PumpPulseFinished,
    AbsorptionFinished { soil_moisture_pct: Option<u8> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrrigationAction {
    SetPump(bool),
    MeasureTank,
    Schedule {
        after_seconds: u32,
        kind: ScheduledEventKind,
    },
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
                self.completed_pulses = 0;
                IrrigationState::Idle
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
                (IrrigationState::CheckingTank, TankState::SensorFault) => {
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
                    actions.push(IrrigationAction::MeasureTank);
                    actions.push(IrrigationAction::Schedule {
                        after_seconds: u32::from(config.absorption_minutes) * 60,
                        kind: ScheduledEventKind::AbsorptionFinished,
                    });
                    IrrigationState::Absorbing { pulse }
                }
                state => state,
            },
            IrrigationEvent::AbsorptionFinished { soil_moisture_pct } => {
                match soil_moisture_pct {
                    None => IrrigationState::Blocked(IrrigationBlockReason::SoilSensorFault),
                    Some(value) if value >= config.target_pct => IrrigationState::Completed,
                    Some(_) if self.completed_pulses >= config.max_pulses_per_cycle => {
                        IrrigationState::Blocked(IrrigationBlockReason::PulseLimitReached)
                    }
                    Some(_) => {
                        actions.push(IrrigationAction::MeasureTank);
                        IrrigationState::CheckingTank
                    }
                }
            }
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
            absorption_minutes: 5,
            max_pulses_per_cycle: 3,
            only_when_lights_off: true,
        }
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
        let decision = machine.handle(
            IrrigationEvent::TankChecked(TankState::WaterLow),
            config(),
        );

        assert_eq!(
            decision.state,
            IrrigationState::Blocked(IrrigationBlockReason::TankLow)
        );
        assert!(!decision.actions.contains(&IrrigationAction::SetPump(true)));
    }

    #[test]
    fn pump_pulse_is_bounded_and_absorption_is_five_minutes() {
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
        assert!(start.actions.contains(&IrrigationAction::SetPump(true)));
        assert!(start.actions.contains(&IrrigationAction::Schedule {
            after_seconds: 8,
            kind: ScheduledEventKind::PumpPulseFinished,
        }));

        let finish = machine.handle(IrrigationEvent::PumpPulseFinished, config());
        assert!(finish.actions.contains(&IrrigationAction::SetPump(false)));
        assert!(finish.actions.contains(&IrrigationAction::MeasureTank));
        assert!(finish.actions.contains(&IrrigationAction::Schedule {
            after_seconds: 300,
            kind: ScheduledEventKind::AbsorptionFinished,
        }));
    }

    #[test]
    fn lights_turning_on_stops_active_pump() {
        let mut machine = IrrigationMachine::default();
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

        let decision = machine.handle(IrrigationEvent::LightsTurnedOn, config());
        assert_eq!(decision.state, IrrigationState::Idle);
        assert_eq!(decision.actions, vec![IrrigationAction::SetPump(false)]);
    }
}
