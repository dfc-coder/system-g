use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use growntrol_protocol::{
    Availability, CommandAcknowledgement, CommandEnvelope, DeviceCommand, DeviceTopics,
    IrrigationStatus, OverrideMode, OverrideTarget, TankStatus, TelemetrySnapshot,
};
use rumqttc::{AsyncClient, Event, Incoming, LastWill, MqttOptions, QoS};

struct SimulatorState {
    sequence: u64,
    lights_on: bool,
    fans_on: bool,
    irrigation: IrrigationStatus,
}

impl Default for SimulatorState {
    fn default() -> Self {
        Self {
            sequence: 0,
            lights_on: true,
            fans_on: false,
            irrigation: IrrigationStatus::Idle,
        }
    }
}

impl SimulatorState {
    fn apply(&mut self, command: &DeviceCommand) {
        match command {
            DeviceCommand::SetOverride { target, mode, .. } => {
                let value = match mode {
                    OverrideMode::Auto => false,
                    OverrideMode::On => true,
                    OverrideMode::Off => false,
                };
                match target {
                    OverrideTarget::Lights => self.lights_on = value,
                    OverrideTarget::Fans => self.fans_on = value,
                }
            }
            DeviceCommand::StartIrrigationCycle => {
                self.irrigation = IrrigationStatus::Pumping;
            }
            DeviceCommand::CancelIrrigation => {
                self.irrigation = IrrigationStatus::Idle;
            }
            DeviceCommand::RequestSnapshot => {}
        }
    }

    fn telemetry(&mut self, device_id: &str) -> TelemetrySnapshot {
        self.sequence += 1;
        let phase = (self.sequence % 10) as f32;

        TelemetrySnapshot {
            schema_version: TelemetrySnapshot::SCHEMA_VERSION,
            device_id: device_id.to_owned(),
            sequence: self.sequence,
            observed_at_unix_ms: Some(unix_time_ms()),
            uptime_seconds: self.sequence * 5,
            clock_ok: true,
            lights_on: self.lights_on,
            fans_on: self.fans_on,
            pump_on: self.irrigation == IrrigationStatus::Pumping,
            temperature_c: Some(24.0 + phase * 0.2),
            humidity_pct: Some(61.0 + phase * 0.3),
            soil_moisture_pct: Some(38 + (self.sequence % 5) as u8),
            tank: TankStatus::WaterAvailable,
            irrigation: self.irrigation,
            fault: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let device_id = env::var("GROWNTROL_DEVICE_ID").unwrap_or_else(|_| "demo-grow-1".to_owned());
    let mqtt_host = env::var("GROWNTROL_MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let mqtt_port = env::var("GROWNTROL_MQTT_PORT")
        .unwrap_or_else(|_| "1883".to_owned())
        .parse()
        .context("parse GROWNTROL_MQTT_PORT")?;
    let topics = DeviceTopics::new(&device_id);

    let offline_payload = serde_json::to_vec(&Availability::Offline)?;
    let mut options = MqttOptions::new(
        format!("growntrol-simulator-{device_id}"),
        mqtt_host,
        mqtt_port,
    );
    options
        .set_keep_alive(Duration::from_secs(15))
        .set_clean_session(false)
        .set_last_will(LastWill::new(
            topics.availability.clone(),
            offline_payload,
            QoS::AtLeastOnce,
            true,
        ));

    let (client, mut event_loop) = AsyncClient::new(options, 32);
    client
        .subscribe(topics.commands.clone(), QoS::AtLeastOnce)
        .await?;
    publish_json(&client, &topics.availability, &Availability::Online, true).await?;

    let mut simulator = SimulatorState::default();
    let mut telemetry_interval = tokio::time::interval(Duration::from_secs(5));
    telemetry_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = telemetry_interval.tick() => {
                let telemetry = simulator.telemetry(&device_id);
                publish_json(&client, &topics.telemetry, &telemetry, true).await?;
            }
            notification = event_loop.poll() => {
                match notification {
                    Ok(Event::Incoming(Incoming::Publish(publish)))
                        if publish.topic == topics.commands =>
                    {
                        let acknowledgement = match serde_json::from_slice::<CommandEnvelope>(&publish.payload) {
                            Ok(envelope) if envelope.schema_version == CommandEnvelope::SCHEMA_VERSION => {
                                simulator.apply(&envelope.command);
                                CommandAcknowledgement {
                                    schema_version: CommandAcknowledgement::SCHEMA_VERSION,
                                    device_id: device_id.clone(),
                                    command_id: envelope.command_id,
                                    accepted: true,
                                    reason: None,
                                    acknowledged_at_unix_ms: Some(unix_time_ms()),
                                }
                            }
                            Ok(envelope) => CommandAcknowledgement {
                                schema_version: CommandAcknowledgement::SCHEMA_VERSION,
                                device_id: device_id.clone(),
                                command_id: envelope.command_id,
                                accepted: false,
                                reason: Some("unsupported command schema version".to_owned()),
                                acknowledged_at_unix_ms: Some(unix_time_ms()),
                            },
                            Err(error) => CommandAcknowledgement {
                                schema_version: CommandAcknowledgement::SCHEMA_VERSION,
                                device_id: device_id.clone(),
                                command_id: "invalid".to_owned(),
                                accepted: false,
                                reason: Some(format!("invalid command payload: {error}")),
                                acknowledged_at_unix_ms: Some(unix_time_ms()),
                            },
                        };
                        publish_json(&client, &topics.acknowledgements, &acknowledgement, false).await?;
                    }
                    Ok(_) => {}
                    Err(error) => {
                        eprintln!("MQTT connection interrupted: {error}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            result = tokio::signal::ctrl_c() => {
                result.context("wait for Ctrl+C")?;
                break;
            }
        }
    }

    publish_json(&client, &topics.availability, &Availability::Offline, true).await?;
    client.disconnect().await?;
    Ok(())
}

async fn publish_json<T: serde::Serialize>(
    client: &AsyncClient,
    topic: &str,
    payload: &T,
    retain: bool,
) -> Result<()> {
    client
        .publish(
            topic,
            QoS::AtLeastOnce,
            retain,
            serde_json::to_vec(payload)?,
        )
        .await
        .with_context(|| format!("publish MQTT message to {topic}"))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
