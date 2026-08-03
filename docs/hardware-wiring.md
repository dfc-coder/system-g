# Hardware wiring — classic ESP32

This document describes the firmware pin contract. Confirm the exact ESP32 board labels before wiring.

## Pin assignment

| Device | ESP32 connection | Notes |
|---|---|---|
| DHT22 data | GPIO4 | Add the pull-up recommended for the sensor/module being used. |
| DS3231 SDA | GPIO21 | I2C, 100 kHz. |
| DS3231 SCL | GPIO22 | I2C, 100 kHz. |
| Capacitive soil sensor | GPIO34 | ADC1 input-only pin. Sensor output must not exceed 3.3 V. |
| Tank float | GPIO32 to GND | Closed means water available; open means low/fault. |
| Lights relay input | GPIO25 | Active-low by default. |
| Fans relay input | GPIO26 | Active-low by default. |
| Pump MOSFET/driver input | GPIO27 | Active-high by default. |

All low-voltage modules must share the ESP32 signal ground unless galvanic isolation explicitly prevents it.

## Pump

The pump must not be powered from an ESP32 GPIO or from the board's 3.3 V regulator. Use:

- a separate correctly sized 5 V supply;
- a logic-level MOSFET or rated driver;
- a flyback diode across the DC pump;
- a common low-voltage ground with the ESP32 driver input;
- a fuse appropriate for the pump and wiring.

## Relays and mains loads

The ESP32 only controls the low-voltage input of an isolated relay, SSR, or contactor. Do not route 220 V wiring on a breadboard.

Before connecting mains loads:

1. Validate every GPIO using LEDs or a multimeter.
2. Verify whether each relay module is active-low or active-high.
3. Confirm the module accepts a 3.3 V control signal.
4. Place mains wiring in a closed, strain-relieved enclosure.
5. Install the required breaker, fuse, residual-current protection, and manual disconnect.
6. Have the mains section assembled or reviewed by a qualified electrician.

## Soil calibration

The firmware currently contains provisional endpoints:

```text
Dry raw: 3200
Wet raw: 1400
```

They are not universal. Record the median raw value in the actual dry substrate and in the chosen wet reference, then replace `SOIL_DRY_RAW` and `SOIL_WET_RAW` in `crates/growntrol-firmware/src/main.rs`.

## Float wiring

The fail-safe default uses the ESP32 pull-up:

```text
GPIO32 ---- float switch ---- GND
```

- Closed circuit: water available.
- Open circuit: tank low or disconnected cable.

Install the float at the minimum level that still keeps the pump inlet submerged.
