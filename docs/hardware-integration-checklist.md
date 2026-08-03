# Hardware integration checklist

Run these checks with mains voltage disconnected. Use LEDs, a multimeter, or low-voltage dummy loads first.

## 1. Build and flash

From `crates/growntrol-firmware`:

```bash
cargo build --release
cargo espflash flash --release --monitor
```

Expected startup log:

```text
Growntrol firmware starting
```

## 2. Safe boot outputs

Reset the ESP32 repeatedly while monitoring GPIO25, GPIO26, and GPIO27.

Pass conditions:

- pump output remains inactive during boot;
- lights and fan relays start inactive;
- no visible relay pulse occurs during reset;
- an invalid or disconnected RTC leaves lights and pump off.

A software-safe boot does not replace physical pull resistors that guarantee inactive driver inputs while the ESP32 is resetting.

## 3. RTC and lighting

1. Set the DS3231 to a time shortly before the configured transition.
2. Confirm the current light state is applied once at boot.
3. Confirm the relay changes once at the transition.
4. Confirm it does not receive repeated writes while the state is unchanged.
5. Disconnect the RTC and confirm lights and pump are forced off.

## 4. DHT22 and fans

1. Keep temperature and humidity below the off thresholds.
2. Raise either value above its on threshold.
3. Confirm the fan relay changes once to on.
4. Keep several consecutive readings above the threshold and confirm the relay does not chatter.
5. Return both readings below the off thresholds and confirm one off transition.
6. Disconnect the sensor and confirm the firmware logs the failure and retries without resetting.

## 5. Soil sensor

1. Record seven-sample median values in dry and wet references.
2. Replace the provisional calibration constants.
3. Confirm a lights-off transition performs one logical soil measurement.
4. Confirm no repeated soil measurements occur while idle.
5. Inject an invalid/disconnected reading and confirm irrigation does not start.

## 6. Tank float

1. Close the float circuit to simulate water available.
2. Open it to simulate tank low or a broken wire.
3. Confirm an open circuit prevents the pump from starting.
4. Complete a test pulse, open the float, and confirm the cycle remains blocked after the absorption timer expires.

## 7. Pump cycle

Use a low-voltage dummy load or the real 5 V pump with the water path supervised.

Pass conditions:

- tank is checked before the pulse;
- normal pulse ends after the configured duration;
- the independent maximum timeout is scheduled at pump start;
- normal completion cancels the maximum timeout;
- tank is checked after the pulse;
- soil is not checked until five minutes have elapsed;
- no more than the configured pulse limit occurs;
- switching lights on stops an active pump.

## 8. Electrical-noise test

With the pump on its final supply:

- run at least 20 short pulses;
- confirm the ESP32 does not reset;
- inspect serial logs for brownout or watchdog resets;
- verify the flyback diode orientation and supply sizing if resets occur.

## Completion evidence

Record:

- board model;
- relay model and polarity;
- pump voltage/current;
- calibrated dry/wet ADC values;
- DS3231 module model;
- serial log for one complete lighting-off and irrigation cycle;
- pass/fail result for every section above.
