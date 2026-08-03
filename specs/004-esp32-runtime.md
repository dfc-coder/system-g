# 004 — ESP32 firmware runtime and hardware adapters

## Objective

Connect the tested domain core to one classic ESP32 without introducing continuous control polling or direct GPIO access outside the adapter layer.

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

- Outputs are driven inactive before sensors and the RTC are initialized.
- The DS3231 determines the wall-clock lighting cycle.
- Monotonic ESP timer time determines deadlines and safety timeouts.
- The next lighting transition is scheduled as a one-shot deadline.
- DHT22 sampling is scheduled for five minutes while stable and one minute near thresholds or while fans are active.
- Soil is measured only after a real lights-on to lights-off transition and after the five-minute absorption deadline.
- The tank float is measured before every pump pulse and after every completed pulse.
- A normal pump pulse and a longer independent pump safety timeout are scheduled together.
- Completing a pulse cancels its safety timeout.
- GPIO is written only when the logical actuator state changes.
- An invalid RTC forces lights and pump off and schedules a retry.

## Adapter rules

- GPIO34 uses ADC1 because Wi-Fi may be enabled later.
- Each logical soil measurement consists of seven ADC samples and uses their median.
- Relay polarity is explicit; the default lights and fans relay configuration is active-low.
- The pump output defaults to active-high for a MOSFET or suitable driver.
- The float input uses a pull-up and treats closed-to-ground as water available.
- A disconnected float wire therefore blocks irrigation instead of reporting water available.

## Acceptance criteria

- Host tests verify the soil median, calibration, transition timing, scheduler replacement/cancellation, and pump safety timeout.
- Xtensa CI compiles the firmware for `xtensa-esp32-espidf` in release mode.
- A repeated fan decision does not write the relay again.
- A pump timeout produces a pump-off action and a terminal blocked irrigation state.
- Hardware integration can be exercised with low-voltage test loads before connecting mains equipment.
