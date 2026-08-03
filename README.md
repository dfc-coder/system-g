# Growntrol

Control and monitoring system for an indoor grow, designed around one ESP32 and developed with Specification-Driven Development and Test-Driven Development.

## Current scope

- Pure Rust domain core, testable without ESP-IDF.
- Scheduled lighting transitions.
- Adaptive DHT22 sampling and fan hysteresis.
- Soil measurement when lights turn off and after watering absorption.
- Tank validation before and after watering.
- Irrigation state machine with bounded pulses.
- ESP32 adapters and local web dashboard in subsequent pull requests.

## Test the domain

```bash
cargo test -p growntrol-core
```

Specifications are stored under `specs/` and traced to tests in each feature module.
