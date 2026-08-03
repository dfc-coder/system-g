# 001 — Growntrol system control

## Objective

Control an indoor grow with one ESP32 while avoiding unnecessary measurements, contradictory commands, and repeated writes to actuators.

## Decisions

- Lighting changes only at configured transitions, configuration changes, clock corrections, and boot.
- DHT22 is sampled every five minutes under normal conditions and every minute near thresholds or while fans are on.
- Fan state is reevaluated after a valid climate reading, but the relay is written only when the desired state changes.
- Soil moisture is measured when lights switch off and five minutes after each watering pulse.
- Tank level is checked immediately before a watering pulse and after the pulse.
- Watering is a finite-state machine; only one irrigation cycle may be active.
- Safety constraints have priority over manual and automatic commands.

## Invariants

- INV-001: the pump never starts when the tank is low or faulty.
- INV-002: automatic irrigation never starts while lights are on.
- INV-003: repeated decisions for the same actuator state produce no physical write.
- INV-004: the pump is controlled by a one-shot pulse deadline.
- INV-005: configuration must pass validation before replacing the active configuration.

## Acceptance criteria

- Default configuration validates successfully.
- Invalid hysteresis and irrigation thresholds are rejected.
- Domain logic is testable on a host computer without ESP-IDF.
- Each later feature PR references this specification and adds tests before adapters.
