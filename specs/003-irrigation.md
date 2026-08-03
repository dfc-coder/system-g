# 003 — Irrigation state machine

## Trigger

- Soil moisture is evaluated when lights switch off.
- If soil is dry, tank level must be measured before the pump can start.

## Cycle

1. Check soil after lights turn off.
2. Check tank.
3. Run one bounded pump pulse.
4. Stop pump and check tank again.
5. Wait five minutes for absorption.
6. Check soil again.
7. Repeat only while below target and below the pulse limit.

## Safety rules

- Tank low or tank sensor failure blocks the pump.
- A missing soil reading blocks irrigation.
- Lights turning on immediately stop an active pump and cancel the cycle.
- Every pump start schedules a one-shot finish deadline.
- Reaching the configured pulse limit blocks further watering.

## Acceptance criteria

- Dry soil requests a tank measurement rather than directly starting the pump.
- Tank low never produces a pump-on action.
- An eight-second pulse is followed by a five-minute absorption deadline.
- Finishing a pulse requests a second tank measurement.
- Lights turning on while pumping produces one pump-off action.
