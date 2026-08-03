use serde::{Deserialize, Serialize};

pub const MQTT_ROOT: &str = "growntrol/v1/devices";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTopics {
    pub availability: String,
    pub telemetry: String,
    pub events: String,
    pub commands: String,
    pub acknowledgements: String,
    pub desired_config: String,
    pub reported_config: String,
}

impl DeviceTopics {
    pub fn new(device_id: &str) -> Self {
        let prefix = format!("{MQTT_ROOT}/{device_id}");
        Self {
            availability: format!("{prefix}/availability"),
            telemetry: format!("{prefix}/telemetry"),
            events: format!("{prefix}/events"),
            commands: format!("{prefix}/commands"),
            acknowledgements: format!("{prefix}/acks"),
            desired_config: format!("{prefix}/config/desired"),
            reported_config: format!("{prefix}/config/reported"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceTopicKind {
    Availability,
    Telemetry,
    Event,
    Command,
    Acknowledgement,
    DesiredConfig,
    ReportedConfig,
}

pub fn parse_device_topic(topic: &str) -> Option<(&str, DeviceTopicKind)> {
    let rest = topic.strip_prefix(MQTT_ROOT)?.strip_prefix('/')?;
    let (device_id, suffix) = rest.split_once('/')?;
    if device_id.is_empty() {
        return None;
    }

    let kind = match suffix {
        "availability" => DeviceTopicKind::Availability,
        "telemetry" => DeviceTopicKind::Telemetry,
        "events" => DeviceTopicKind::Event,
        "commands" => DeviceTopicKind::Command,
        "acks" => DeviceTopicKind::Acknowledgement,
        "config/desired" => DeviceTopicKind::DesiredConfig,
        "config/reported" => DeviceTopicKind::ReportedConfig,
        _ => return None,
    };

    Some((device_id, kind))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Online,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TankStatus {
    Unknown,
    WaterAvailable,
    WaterLow,
    SensorFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IrrigationStatus {
    Idle,
    CheckingTank,
    Pumping,
    Absorbing,
    Complete,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub schema_version: u16,
    pub device_id: String,
    pub sequence: u64,
    pub observed_at_unix_ms: Option<u64>,
    pub uptime_seconds: u64,
    pub clock_ok: bool,
    pub lights_on: bool,
    pub fans_on: bool,
    pub pump_on: bool,
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub soil_moisture_pct: Option<u8>,
    pub tank: TankStatus,
    pub irrigation: IrrigationStatus,
    pub fault: Option<String>,
}

impl TelemetrySnapshot {
    pub const SCHEMA_VERSION: u16 = 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideTarget {
    Lights,
    Fans,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverrideMode {
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeviceCommand {
    SetOverride {
        target: OverrideTarget,
        mode: OverrideMode,
        duration_seconds: Option<u32>,
    },
    StartIrrigationCycle,
    CancelIrrigation,
    RequestSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub schema_version: u16,
    pub command_id: String,
    pub issued_at_unix_ms: u64,
    pub command: DeviceCommand,
}

impl CommandEnvelope {
    pub const SCHEMA_VERSION: u16 = 1;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAcknowledgement {
    pub schema_version: u16,
    pub device_id: String,
    pub command_id: String,
    pub accepted: bool,
    pub reason: Option<String>,
    pub acknowledged_at_unix_ms: Option<u64>,
}

impl CommandAcknowledgement {
    pub const SCHEMA_VERSION: u16 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_stable_versioned_topics() {
        let topics = DeviceTopics::new("grow-room-1");
        assert_eq!(
            topics.telemetry,
            "growntrol/v1/devices/grow-room-1/telemetry"
        );
        assert_eq!(
            topics.desired_config,
            "growntrol/v1/devices/grow-room-1/config/desired"
        );
    }

    #[test]
    fn parses_known_device_topics() {
        assert_eq!(
            parse_device_topic("growntrol/v1/devices/esp32-a/acks"),
            Some(("esp32-a", DeviceTopicKind::Acknowledgement))
        );
        assert_eq!(
            parse_device_topic("growntrol/v1/devices/esp32-a/config/reported"),
            Some(("esp32-a", DeviceTopicKind::ReportedConfig))
        );
    }

    #[test]
    fn rejects_unknown_or_incomplete_topics() {
        assert_eq!(parse_device_topic("growntrol/v1/devices//telemetry"), None);
        assert_eq!(
            parse_device_topic("growntrol/v1/devices/esp32-a/unknown"),
            None
        );
    }

    #[test]
    fn command_round_trip_is_stable() {
        let command = CommandEnvelope {
            schema_version: CommandEnvelope::SCHEMA_VERSION,
            command_id: "cmd-1".to_owned(),
            issued_at_unix_ms: 1_700_000_000_000,
            command: DeviceCommand::SetOverride {
                target: OverrideTarget::Fans,
                mode: OverrideMode::Auto,
                duration_seconds: None,
            },
        };

        let json = serde_json::to_string(&command).expect("serialize command");
        let decoded: CommandEnvelope = serde_json::from_str(&json).expect("deserialize command");
        assert_eq!(decoded, command);
    }
}
