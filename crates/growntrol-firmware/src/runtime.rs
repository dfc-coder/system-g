use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use growntrol_core::{
    evaluate_fan, evaluate_lighting, seconds_until_minute, DeadlineScheduler, FanState,
    IrrigationAction, IrrigationBlockReason, IrrigationEvent, IrrigationMachine,
    ScheduledEventKind, SystemConfig, TankState,
};
use log::{error, info, warn};

use crate::ports::{
    Actuator, ClimateSensor, HardwareEvent, PumpActuator, SoilSensor, TankSensor, WallClock,
};

pub struct Runtime<'a> {
    config: SystemConfig,
    scheduler: DeadlineScheduler,
    irrigation: IrrigationMachine,
    hardware_events: Receiver<HardwareEvent>,
    lights: &'a mut dyn Actuator,
    fans: &'a mut dyn Actuator,
    pump: &'a mut dyn PumpActuator,
    climate: &'a mut dyn ClimateSensor,
    soil: &'a mut dyn SoilSensor,
    tank: &'a mut dyn TankSensor,
    clock: &'a mut dyn WallClock,
}

impl<'a> Runtime<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: SystemConfig,
        hardware_events: Receiver<HardwareEvent>,
        lights: &'a mut dyn Actuator,
        fans: &'a mut dyn Actuator,
        pump: &'a mut dyn PumpActuator,
        climate: &'a mut dyn ClimateSensor,
        soil: &'a mut dyn SoilSensor,
        tank: &'a mut dyn TankSensor,
        clock: &'a mut dyn WallClock,
    ) -> Result<Self> {
        config.validate().context("invalid system configuration")?;
        Ok(Self {
            config,
            scheduler: DeadlineScheduler::default(),
            irrigation: IrrigationMachine::default(),
            hardware_events,
            lights,
            fans,
            pump,
            climate,
            soil,
            tank,
            clock,
        })
    }

    pub fn run_forever(&mut self) -> Result<()> {
        self.fail_safe_outputs()?;
        let now = monotonic_seconds();
        self.scheduler
            .schedule_after(now, 1, ScheduledEventKind::ClimateSample);
        self.reconcile_lighting(false)?;

        loop {
            self.process_due_events()?;

            let now = monotonic_seconds();
            let wait = self
                .scheduler
                .next_due_at()
                .map(|deadline| Duration::from_secs(deadline.saturating_sub(now)))
                .unwrap_or_else(|| Duration::from_secs(60));

            match self.hardware_events.recv_timeout(wait) {
                Ok(event) => self.handle_hardware_event(event)?,
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(anyhow!("hardware event channel disconnected"));
                }
            }
        }
    }

    fn process_due_events(&mut self) -> Result<()> {
        let due = self.scheduler.take_due(monotonic_seconds());
        for event in due {
            if let Err(event_error) = self.handle_scheduled(event) {
                error!("scheduled event {event:?} failed: {event_error:#}");
                self.handle_irrigation(IrrigationEvent::Abort(
                    IrrigationBlockReason::HardwareFault,
                ))?;
            }
        }
        Ok(())
    }

    fn fail_safe_outputs(&mut self) -> Result<()> {
        self.pump.disarm_safety_timeout()?;
        self.pump.set(false)?;
        self.lights.set(false)?;
        self.fans.set(false)?;
        Ok(())
    }

    fn handle_hardware_event(&mut self, event: HardwareEvent) -> Result<()> {
        match event {
            HardwareEvent::PumpSafetyTimeout => {
                warn!("independent pump safety timer expired");
                self.handle_irrigation(IrrigationEvent::PumpSafetyTimeout)
            }
        }
    }

    fn handle_scheduled(&mut self, event: ScheduledEventKind) -> Result<()> {
        match event {
            ScheduledEventKind::LightTransition => self.reconcile_lighting(true),
            ScheduledEventKind::ClimateSample => self.sample_climate(),
            ScheduledEventKind::PumpPulseFinished => {
                self.handle_irrigation(IrrigationEvent::PumpPulseFinished)
            }
            ScheduledEventKind::AbsorptionFinished => {
                let moisture = self.soil.read_percent().ok();
                self.handle_irrigation(IrrigationEvent::AbsorptionFinished {
                    soil_moisture_pct: moisture,
                })
            }
        }
    }

    fn reconcile_lighting(&mut self, trigger_irrigation: bool) -> Result<()> {
        let date_time = match self.clock.now() {
            Ok(value) => value,
            Err(clock_error) => {
                warn!("RTC unavailable; forcing lights and irrigation off: {clock_error:#}");
                self.lights.set(false)?;
                self.handle_irrigation(IrrigationEvent::Abort(
                    IrrigationBlockReason::ClockFault,
                ))?;
                self.scheduler.schedule_after(
                    monotonic_seconds(),
                    60,
                    ScheduledEventKind::LightTransition,
                );
                return Ok(());
            }
        };

        let decision = evaluate_lighting(date_time.minute_of_day(), self.config.lighting);
        let was_on = self.lights.is_on();
        let changed = self.lights.set(decision.lights_on)?;
        if changed {
            info!(
                "lights changed: {}",
                if decision.lights_on { "on" } else { "off" }
            );
        }

        let after = seconds_until_minute(
            date_time.hour,
            date_time.minute,
            date_time.second,
            decision.next_transition_minute,
        );
        self.scheduler.schedule_after(
            monotonic_seconds(),
            after,
            ScheduledEventKind::LightTransition,
        );

        if !trigger_irrigation || !changed {
            return Ok(());
        }

        if was_on && !decision.lights_on {
            let moisture = self.soil.read_percent().ok();
            self.handle_irrigation(IrrigationEvent::LightsTurnedOff {
                soil_moisture_pct: moisture,
            })?;
        } else if !was_on && decision.lights_on {
            self.handle_irrigation(IrrigationEvent::LightsTurnedOn)?;
        }

        Ok(())
    }

    fn sample_climate(&mut self) -> Result<()> {
        let now = monotonic_seconds();
        match self.climate.read() {
            Ok(reading) => {
                let current = if self.fans.is_on() {
                    FanState::On
                } else {
                    FanState::Off
                };
                let decision = evaluate_fan(current, reading, self.config.fan);
                if decision.write_relay {
                    self.fans.set(decision.desired_state == FanState::On)?;
                    info!(
                        "fans changed: {:?}, temperature={:.1}, humidity={:.1}",
                        decision.desired_state, reading.temperature_c, reading.humidity_pct
                    );
                }
                self.scheduler.schedule_after(
                    now,
                    u32::from(decision.next_sample_minutes) * 60,
                    ScheduledEventKind::ClimateSample,
                );
            }
            Err(sensor_error) => {
                warn!("DHT22 sample failed: {sensor_error:#}");
                self.scheduler.schedule_after(
                    now,
                    u32::from(self.config.fan.alert_sample_minutes) * 60,
                    ScheduledEventKind::ClimateSample,
                );
            }
        }
        Ok(())
    }

    fn handle_irrigation(&mut self, initial_event: IrrigationEvent) -> Result<()> {
        let mut pending = VecDeque::from([initial_event]);

        while let Some(event) = pending.pop_front() {
            let decision = self.irrigation.handle(event, self.config.irrigation);
            info!("irrigation state: {:?}", decision.state);

            for action in decision.actions {
                match action {
                    IrrigationAction::SetPump(on) => {
                        if on && self.lights.is_on() {
                            warn!("pump-on action rejected because lights are on");
                            self.pump.disarm_safety_timeout()?;
                            pending.push_back(IrrigationEvent::Abort(
                                IrrigationBlockReason::LightsOn,
                            ));
                            break;
                        }
                        if self.pump.set(on)? {
                            info!("pump changed: {}", if on { "on" } else { "off" });
                        }
                    }
                    IrrigationAction::ArmPumpSafety { after_seconds } => {
                        self.pump
                            .arm_safety_timeout(Duration::from_secs(u64::from(after_seconds)))?;
                    }
                    IrrigationAction::DisarmPumpSafety => {
                        self.pump.disarm_safety_timeout()?;
                    }
                    IrrigationAction::MeasureTank => {
                        let state = self.tank.read().unwrap_or(TankState::SensorFault);
                        pending.push_back(IrrigationEvent::TankChecked(state));
                    }
                    IrrigationAction::Schedule {
                        after_seconds,
                        kind,
                    } => {
                        self.scheduler
                            .schedule_after(monotonic_seconds(), after_seconds, kind);
                    }
                    IrrigationAction::CancelScheduled(kind) => self.scheduler.cancel(kind),
                }
            }
        }

        Ok(())
    }
}

fn monotonic_seconds() -> u64 {
    let micros = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
    u64::try_from(micros.max(0)).unwrap_or(0) / 1_000_000
}
