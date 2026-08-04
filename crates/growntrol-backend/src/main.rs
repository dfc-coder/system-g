use std::collections::HashMap;
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use growntrol_protocol::{
    parse_device_topic, Availability, CommandAcknowledgement, CommandEnvelope, DeviceCommand,
    DeviceTopicKind, DeviceTopics, TelemetrySnapshot, MQTT_ROOT,
};
use rumqttc::{AsyncClient, Event, EventLoop, Incoming, MqttOptions, QoS};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct Settings {
    bind: SocketAddr,
    mqtt_host: String,
    mqtt_port: u16,
    mqtt_client_id: String,
    mqtt_username: Option<String>,
    mqtt_password: Option<String>,
}

impl Settings {
    fn from_env() -> Result<Self> {
        let bind = env::var("GROWNTROL_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse()
            .context("parse GROWNTROL_BIND")?;
        let mqtt_port = env::var("GROWNTROL_MQTT_PORT")
            .unwrap_or_else(|_| "1883".to_owned())
            .parse()
            .context("parse GROWNTROL_MQTT_PORT")?;

        Ok(Self {
            bind,
            mqtt_host: env::var("GROWNTROL_MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned()),
            mqtt_port,
            mqtt_client_id: env::var("GROWNTROL_MQTT_CLIENT_ID")
                .unwrap_or_else(|_| "growntrol-backend".to_owned()),
            mqtt_username: env::var("GROWNTROL_MQTT_USERNAME").ok(),
            mqtt_password: env::var("GROWNTROL_MQTT_PASSWORD").ok(),
        })
    }
}

#[derive(Debug, Clone, Serialize)]
struct DeviceRecord {
    device_id: String,
    availability: Availability,
    telemetry: Option<TelemetrySnapshot>,
    latest_acknowledgement: Option<CommandAcknowledgement>,
    latest_event: Option<Value>,
    last_seen_unix_ms: u64,
}

impl DeviceRecord {
    fn new(device_id: &str) -> Self {
        Self {
            device_id: device_id.to_owned(),
            availability: Availability::Offline,
            telemetry: None,
            latest_acknowledgement: None,
            latest_event: None,
            last_seen_unix_ms: unix_time_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BackendEvent {
    Availability {
        device_id: String,
        availability: Availability,
    },
    Telemetry {
        device_id: String,
        telemetry: TelemetrySnapshot,
    },
    Acknowledgement {
        device_id: String,
        acknowledgement: CommandAcknowledgement,
    },
    DeviceEvent {
        device_id: String,
        payload: Value,
    },
}

#[derive(Clone)]
struct AppState {
    devices: Arc<RwLock<HashMap<String, DeviceRecord>>>,
    mqtt: AsyncClient,
    events: broadcast::Sender<BackendEvent>,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "growntrol_backend=info".into()),
        )
        .init();

    let settings = Settings::from_env()?;
    let (mqtt, event_loop) = mqtt_client(&settings);
    let (events, _) = broadcast::channel(256);
    let state = AppState {
        devices: Arc::new(RwLock::new(HashMap::new())),
        mqtt,
        events,
    };

    tokio::spawn(run_mqtt(event_loop, state.clone()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/devices", get(list_devices))
        .route("/api/devices/{device_id}", get(get_device))
        .route("/api/devices/{device_id}/commands", post(publish_command))
        .route("/api/events", get(event_stream))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(settings.bind)
        .await
        .with_context(|| format!("bind backend to {}", settings.bind))?;
    info!(address = %settings.bind, "Growntrol backend listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve Growntrol backend")
}

fn mqtt_client(settings: &Settings) -> (AsyncClient, EventLoop) {
    let mut options = MqttOptions::new(
        settings.mqtt_client_id.clone(),
        settings.mqtt_host.clone(),
        settings.mqtt_port,
    );
    options.set_keep_alive(Duration::from_secs(30));
    options.set_clean_session(false);

    if let Some(username) = &settings.mqtt_username {
        options.set_credentials(
            username.clone(),
            settings.mqtt_password.clone().unwrap_or_default(),
        );
    }

    AsyncClient::new(options, 64)
}

async fn run_mqtt(mut event_loop: EventLoop, state: AppState) {
    for topic in [
        format!("{MQTT_ROOT}/+/availability"),
        format!("{MQTT_ROOT}/+/telemetry"),
        format!("{MQTT_ROOT}/+/acks"),
        format!("{MQTT_ROOT}/+/events"),
    ] {
        if let Err(error) = state.mqtt.subscribe(&topic, QoS::AtLeastOnce).await {
            error!(%topic, %error, "failed to subscribe to MQTT topic");
            return;
        }
    }

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                if let Err(error) = process_publish(&state, &publish.topic, &publish.payload).await
                {
                    warn!(topic = %publish.topic, %error, "ignored invalid MQTT message");
                }
            }
            Ok(_) => {}
            Err(error) => {
                warn!(%error, "MQTT connection interrupted; retrying");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_publish(state: &AppState, topic: &str, payload: &[u8]) -> Result<()> {
    let (device_id, kind) = parse_device_topic(topic).context("unrecognized device topic")?;
    let now = unix_time_ms();

    let event = match kind {
        DeviceTopicKind::Availability => {
            let availability = parse_availability(payload)?;
            let mut devices = state.devices.write().await;
            let record = devices
                .entry(device_id.to_owned())
                .or_insert_with(|| DeviceRecord::new(device_id));
            record.availability = availability;
            record.last_seen_unix_ms = now;
            BackendEvent::Availability {
                device_id: device_id.to_owned(),
                availability,
            }
        }
        DeviceTopicKind::Telemetry => {
            let telemetry: TelemetrySnapshot =
                serde_json::from_slice(payload).context("decode telemetry JSON")?;
            anyhow::ensure!(
                telemetry.device_id == device_id,
                "telemetry device_id does not match topic"
            );
            anyhow::ensure!(
                telemetry.schema_version == TelemetrySnapshot::SCHEMA_VERSION,
                "unsupported telemetry schema version"
            );

            let mut devices = state.devices.write().await;
            let record = devices
                .entry(device_id.to_owned())
                .or_insert_with(|| DeviceRecord::new(device_id));
            record.availability = Availability::Online;
            record.telemetry = Some(telemetry.clone());
            record.last_seen_unix_ms = now;
            BackendEvent::Telemetry {
                device_id: device_id.to_owned(),
                telemetry,
            }
        }
        DeviceTopicKind::Acknowledgement => {
            let acknowledgement: CommandAcknowledgement =
                serde_json::from_slice(payload).context("decode acknowledgement JSON")?;
            anyhow::ensure!(
                acknowledgement.device_id == device_id,
                "acknowledgement device_id does not match topic"
            );

            let mut devices = state.devices.write().await;
            let record = devices
                .entry(device_id.to_owned())
                .or_insert_with(|| DeviceRecord::new(device_id));
            record.latest_acknowledgement = Some(acknowledgement.clone());
            record.last_seen_unix_ms = now;
            BackendEvent::Acknowledgement {
                device_id: device_id.to_owned(),
                acknowledgement,
            }
        }
        DeviceTopicKind::Event => {
            let payload: Value = serde_json::from_slice(payload).context("decode event JSON")?;
            let mut devices = state.devices.write().await;
            let record = devices
                .entry(device_id.to_owned())
                .or_insert_with(|| DeviceRecord::new(device_id));
            record.latest_event = Some(payload.clone());
            record.last_seen_unix_ms = now;
            BackendEvent::DeviceEvent {
                device_id: device_id.to_owned(),
                payload,
            }
        }
        DeviceTopicKind::Command
        | DeviceTopicKind::DesiredConfig
        | DeviceTopicKind::ReportedConfig => return Ok(()),
    };

    let _ = state.events.send(event);
    Ok(())
}

fn parse_availability(payload: &[u8]) -> Result<Availability> {
    if let Ok(value) = serde_json::from_slice(payload) {
        return Ok(value);
    }

    match std::str::from_utf8(payload)?.trim() {
        "online" => Ok(Availability::Online),
        "offline" => Ok(Availability::Offline),
        other => anyhow::bail!("unknown availability value: {other}"),
    }
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_devices(State(state): State<AppState>) -> Json<Vec<DeviceRecord>> {
    let devices = state.devices.read().await;
    let mut records: Vec<_> = devices.values().cloned().collect();
    records.sort_by(|left, right| left.device_id.cmp(&right.device_id));
    Json(records)
}

async fn get_device(
    Path(device_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DeviceRecord>, StatusCode> {
    state
        .devices
        .read()
        .await
        .get(&device_id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn publish_command(
    Path(device_id): Path<String>,
    State(state): State<AppState>,
    Json(command): Json<DeviceCommand>,
) -> Result<(StatusCode, Json<CommandEnvelope>), (StatusCode, String)> {
    let envelope = CommandEnvelope {
        schema_version: CommandEnvelope::SCHEMA_VERSION,
        command_id: Uuid::new_v4().to_string(),
        issued_at_unix_ms: unix_time_ms(),
        command,
    };
    let payload = serde_json::to_vec(&envelope).map_err(internal_error)?;
    let topics = DeviceTopics::new(&device_id);

    state
        .mqtt
        .publish(topics.commands, QoS::AtLeastOnce, false, payload)
        .await
        .map_err(internal_error)?;

    Ok((StatusCode::ACCEPTED, Json(envelope)))
}

async fn event_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<SseEvent, Infallible>>> {
    let mut receiver = state.events.subscribe();
    let stream = async_stream::stream! {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    match SseEvent::default().json_data(event) {
                        Ok(event) => yield Ok(event),
                        Err(error) => warn!(%error, "failed to serialize SSE event"),
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    warn!(skipped, "SSE client lagged behind");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    error!(%error, "backend request failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal backend error".to_owned(),
    )
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C signal handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
