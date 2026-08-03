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
- Event/deadline runtime that blocks until a hardware event or the nearest deadline.
- Separate host CI and Xtensa ESP32 firmware CI.

The local web dashboard, persistent configuration, Wi-Fi, and MQTT remain subsequent phases.

## Repository layout

```text
crates/growntrol-core/       Host-testable domain rules
crates/growntrol-firmware/   ESP-IDF firmware for classic ESP32
specs/                       SDD specifications
docs/                        Wiring and hardware integration procedures
```

## Test the domain

```bash
cargo test -p growntrol-core
```

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

A successful CI build proves source compatibility with the selected Rust, ESP-IDF, and classic ESP32 target. It does not prove physical wiring, relay polarity, signal voltage, sensor calibration, electrical-noise immunity, or mains safety. Those items require the documented low-voltage hardware integration procedure.
