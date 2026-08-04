# 005 — MQTT, backend, and dashboard vertical slice

## Objective

Expose Growntrol state through a versioned MQTT contract, aggregate the latest device state in a local backend, and present that state and safe commands in a browser dashboard.

This phase establishes the platform boundary before the ESP32 Wi-Fi/MQTT transport is enabled. A simulator or future firmware client can publish the same protocol without changing the backend or dashboard.

The dashboard is maintained independently in `dfc-coder/growntrol-web`. This repository owns firmware, protocol, backend, simulator, broker configuration, and orchestration only.

## Architecture

```text
ESP32 or simulator
      │ MQTT 3.1.1 / QoS 1
      ▼
Mosquitto broker
      │
      ▼
Rust backend (rumqttc + Axum)
      │ HTTP JSON + SSE
      ▼
growntrol-web (Vue 3 / Vite)
```

The broker is transport only. The backend does not replace the ESP32 control engine and does not drive GPIO directly.

## Repository boundary

- `system-g` owns the MQTT contract, backend API, simulator, broker, firmware, and Podman Compose orchestration.
- `growntrol-web` owns Vue, Vite, TypeScript, Nginx, frontend CI, and the dashboard image definition.
- The local default layout places both repositories as siblings.
- `GROWNTROL_WEB_CONTEXT` may override the dashboard build context for CI or a different checkout layout.
- The dashboard reads an optional `VITE_API_BASE_URL`; an empty value uses the same-origin Nginx or Vite proxy.

## MQTT namespace

All topics are versioned beneath:

```text
growntrol/v1/devices/{device_id}
```

| Topic suffix | Direction | Retained | Purpose |
|---|---|---:|---|
| `availability` | device → broker | yes | Online/offline state and last will |
| `telemetry` | device → broker | yes | Latest complete state snapshot |
| `events` | device → broker | no | Significant transitions and faults |
| `commands` | backend → device | no | Typed command envelopes |
| `acks` | device → backend | no | Accepted/rejected command result |
| `config/desired` | backend → device | yes | Future desired configuration |
| `config/reported` | device → backend | yes | Future applied configuration |

Telemetry and commands use JSON with `schema_version = 1`.

## Command safety

- MQTT callbacks never write GPIO.
- A firmware MQTT adapter must deserialize, validate, and enqueue commands into the central control runtime.
- Lights and fan overrides must expire unless returned to `auto` earlier.
- `start_irrigation_cycle` is a request, not an unconditional pump command.
- The firmware remains responsible for light state, tank state, soil state, active-cycle exclusion, pulse limits, and pump watchdog enforcement.
- No protocol command can bypass the physical emergency stop or pump maximum-runtime watchdog.
- Every command carries a unique `command_id` and requires an acknowledgement.

## Backend behavior

- Subscribe to availability, telemetry, event, and acknowledgement topics for all devices.
- Reject telemetry whose payload `device_id` does not match its MQTT topic.
- Reject unsupported telemetry schema versions.
- Keep the latest state in memory for this vertical slice.
- Publish commands with QoS 1.
- Expose:
  - `GET /health`
  - `GET /api/devices`
  - `GET /api/devices/{device_id}`
  - `POST /api/devices/{device_id}/commands`
  - `GET /api/events` as Server-Sent Events.
- A later persistence phase may add SQLite without changing the MQTT or HTTP contract.

## Dashboard behavior

- Show all discovered devices and online/offline state.
- Show temperature, humidity, soil moisture, tank state, irrigation state, faults, and actuator state when telemetry is available.
- Refresh from SSE rather than polling continuously.
- Provide lights and fans controls for `auto`, `on`, and `off`.
- Non-auto overrides request a bounded 30-minute duration.
- Provide irrigation start, irrigation cancellation, and snapshot request actions.
- Clearly state that firmware safety rules remain authoritative.

## Acceptance criteria

- Protocol topic parsing and command JSON round trips are covered by Rust tests.
- Backend formatting, compilation, and tests pass on stable Rust.
- The independent `growntrol-web` CI builds the frontend and its Podman-compatible OCI image.
- A local Mosquitto broker, backend, external dashboard checkout, and simulator can be started without committing credentials.
- Publishing a telemetry snapshot causes the device to appear through the HTTP API and dashboard.
- Posting a dashboard command publishes a versioned command envelope on the matching device topic.
- No backend or dashboard code contains direct hardware-control logic.
- ESP32 Wi-Fi and MQTT transport remains a separate, physically validated follow-up feature.
