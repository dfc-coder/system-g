# 002 — Lighting and climate control

## Objective

Control the configured lighting cycle and ventilators without continuous polling or repeated relay writes.

## Requirements

- Lighting is evaluated at boot, configuration changes, clock corrections, and scheduled transitions.
- A cycle may cross midnight.
- Only the next lighting transition is scheduled.
- Climate is sampled every five minutes while stable.
- Climate is sampled every minute near a fan threshold or while fans are active.
- Fan decisions use separate on/off thresholds for hysteresis.
- A relay write occurs only when the desired fan state differs from the applied state.

## Acceptance criteria

- A 20:00–14:00 cycle is on at 23:00 and 08:00, and off at 15:00.
- The cycle turns off exactly at 14:00.
- Repeated high-temperature measurements produce one transition from off to on.
- Measurements inside the hysteresis band preserve the current state.
- Stable conditions return a five-minute sampling interval.
- Near-threshold or active-fan conditions return a one-minute sampling interval.
