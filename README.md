# Growntrol

Control and monitoring system for an indoor grow, designed around one classic ESP32 and developed with Specification-Driven Development and Test-Driven Development.

## Implemented scope

- Pure Rust domain core, testable without ESP-IDF.
- Scheduled lighting transitions driven by a DS3231 RTC.
- DHT22 sampling every five minutes while stable and every minute near thresholds or while fans are active.
- Fan hysteresis with relay writes only on state transitions.
- Soil measurement when lights turn off and five minutes after watering.
- Seven-sample raw ADC median filtering and configurable soil calibration.
- Tank validation before and after every watering pulse.
- Irrigation state machine with bounded pulses and an independent ESP timer pump watchdog.
- ESP32 adapters for GPIO outputs, DHT22, ADC1 soil input, tank float, and DS3231.
- Current ESP-IDF `esp_adc/adc_oneshot.h` and `driver/i2c_master.h` APIs instead of deprecated legacy ADC/I2C drivers.
- Event/deadline runtime that blocks until a hardware event or the nearest deadline.
- Versioned MQTT topics and JSON payloads shared by devices and services.
- Rust backend that aggregates MQTT state and exposes HTTP JSON and Server-Sent Events.
- Vue 3 dashboard for live state, bounded overrides, and safe irrigation requests.
- Optional MQTT device simulator for end-to-end testing without physical sensors.
- Local Mosquitto, backend, dashboard, and demo-device stack through Docker Compose.
- Separate host, firmware, platform, and dashboard CI checks.

ESP32 Wi-Fi provisioning and its MQTT transport adapter remain a separate hardware-validated phase. The backend and dashboard never drive GPIO directly.

## Repository layout

```text
apps/growntrol-dashboard/     Vue 3 and Vite operational dashboard
crates/growntrol-backend/     MQTT-to-HTTP/SSE Rust backend
crates/growntrol-core/        Host-testable domain rules
crates/growntrol-firmware/    ESP-IDF firmware for classic ESP32
crates/growntrol-protocol/    Shared MQTT topics and payload contracts
crates/growntrol-simulator/   End-to-end MQTT demonstration device
deploy/                       Local broker configuration
specs/                        SDD specifications
docs/                         Wiring and hardware integration procedures
```

## Test the Rust code

```bash
cargo test \
  -p growntrol-core \
  -p growntrol-protocol \
  -p growntrol-backend \
  -p growntrol-simulator
```

## Build the dashboard

```bash
cd apps/growntrol-dashboard
npm install
npm run build
```

Node.js 20.19 or newer is required by the selected Vite version.

## Run the local platform

Start the broker, backend, and dashboard:

```bash
docker compose up --build
```

Start the same platform with a simulated Growntrol device:

```bash
docker compose --profile demo up --build
```

The simulator publishes retained availability and telemetry, receives dashboard commands, and publishes acknowledgements. It does not represent physical safety validation or replace the ESP32 control engine.

Services:

- MQTT broker: `localhost:1883`
- Backend API and SSE: `http://localhost:8080`
- Dashboard: `http://localhost:5173`

The included Mosquitto configuration permits anonymous access only for local development. A network-exposed deployment must use authentication, authorization, and TLS.

## MQTT contract

Device topics are rooted at:

```text
growntrol/v1/devices/{device_id}
```

The detailed topic directions, retention rules, payloads, and command safety boundary are defined in `specs/005-mqtt-backend-dashboard.md`.

## Build and flash the ESP32 firmware

```bash
cd crates/growntrol-firmware
cargo build --release
cargo espflash flash --release --monitor
```

Before connecting loads, read:

- `docs/hardware-wiring.md`
- `docs/hardware-integration-checklist.md`

The soil calibration values in the firmware are provisional and must be measured with the actual sensor and substrate.

A successful CI build proves source compatibility with the selected Rust, ESP-IDF, and classic ESP32 target. A clean hardware smoke test must additionally confirm that startup no longer emits the ESP-IDF legacy I2C or legacy ADC driver warnings. Neither result proves physical wiring, relay polarity, signal voltage, sensor calibration, electrical-noise immunity, or mains safety; those items require the documented low-voltage integration procedure.
