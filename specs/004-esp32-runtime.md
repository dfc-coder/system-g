# 004 — ESP32 firmware runtime and hardware adapters

## Objective

Connect the tested domain core to one classic ESP32 without introducing continuous control polling or direct actuator access outside the adapter layer.

## Hardware map

| Function | GPIO |
|---|---:|
| DHT22 data | 4 |
| DS3231 SDA | 21 |
| DS3231 SCL | 22 |
| Soil ADC1 | 34 |
| Tank float | 32 |
| Lights relay | 25 |
| Fans relay | 26 |
| Pump driver | 27 |

## Runtime rules

- Outputs are preloaded and configured inactive before sensors and the RTC are initialized.
- The DS3231 determines the wall-clock lighting cycle.
- Monotonic ESP timer time determines relative deadlines.
- The runtime blocks on a hardware-event channel until the nearest deadline instead of executing a periodic control loop.
- The next lighting transition is scheduled as a one-shot deadline.
- DHT22 sampling is scheduled for five minutes while stable and one minute near thresholds or while fans are active.
- Soil is measured only after a real lights-on to lights-off transition and after the five-minute absorption deadline.
- The tank float is measured before every pump pulse and after every completed pulse.
- A normal pump pulse uses the domain deadline scheduler.
- A separate ESP-IDF one-shot timer is armed before energizing the pump as an independent maximum-runtime watchdog.
- The watchdog callback directly drives the pump GPIO inactive and posts a `PumpSafetyTimeout` hardware event to reconcile domain state.
- Completing a pulse disarms the watchdog.
- GPIO is written only when the logical actuator state changes.
- An invalid RTC forces lights off, aborts irrigation, and schedules a bounded retry.
- A runtime or adapter fault aborts irrigation and cancels pending pump and absorption work.

## Adapter rules

- GPIO34 uses ADC1 because Wi-Fi may be enabled later.
- Each logical soil measurement consists of seven raw ADC samples and uses their median.
- Soil calibration endpoints are raw ADC values, not calibrated millivolts.
- Relay polarity is explicit; the default lights and fans relay configuration is active-low.
- The pump output defaults to active-high for a MOSFET or suitable driver.
- The float input uses a pull-up and treats closed-to-ground as water available.
- A disconnected float wire therefore blocks irrigation instead of reporting water available.
- DS3231 transactions have a finite 100 ms I2C timeout.

## Acceptance criteria

- Host tests verify soil median filtering, raw calibration, transition timing, scheduler replacement/cancellation, pump watchdog ordering, and terminal fault handling.
- Xtensa CI formats and compiles the firmware for `xtensa-esp32-espidf` in release mode.
- A repeated fan decision does not write the relay again.
- The watchdog is armed before the pump is energized.
- A pump timeout de-energizes the physical output and produces a terminal blocked irrigation state.
- Hardware integration is validated with low-voltage test loads before connecting mains equipment.
