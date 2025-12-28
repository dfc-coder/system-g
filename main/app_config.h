#pragma once

#include "driver/i2c_master.h"
#include "driver/gpio.h"

/* TAG */
#define APP_TAG "grow_rtc"

/* I2C */
#define I2C_PORT      I2C_NUM_0
#define I2C_SDA_GPIO  GPIO_NUM_21
#define I2C_SCL_GPIO  GPIO_NUM_22
#define I2C_FREQ_HZ   100000

/* OLED SH1106 */
#define OLED_ADDR     0x3C
#define OLED_W        128
#define OLED_H        64
#define OLED_PAGES    (OLED_H / 8)

/* DS3231 */
#define RTC_ADDR      0x68

/* Encoder */
#define ENC_A_GPIO      GPIO_NUM_32
#define ENC_B_GPIO      GPIO_NUM_33
#define ENC_BTN_GPIO    GPIO_NUM_25

#define ENC_EDGES_PER_STEP        4
#define ENC_DIR_INVERT            0
#define ENC_GLITCH_NS             5000

#define ENC_AB_PULLUP             1
#define ENC_AB_PULLDOWN           0
#define ENC_MAX_EDGES_PER_SAMPLE  24

/* Button debounce */
#define BTN_POLL_MS         10
#define BTN_DEBOUNCE_MS     30

/* Tasks */
#define INPUT_TASK_MS       10
#define UI_MIN_REDRAW_MS    16
#define UI_MS_PER_FRAME     500
#define CONTROL_TASK_MS     1000

/* Task priorities */
#define PRIO_UI     4
#define PRIO_INPUT  2
#define PRIO_CTL    3

/* Relay */
#define RELAY_GPIO        GPIO_NUM_16
#define RELAY_ACTIVE_LOW  1

/* Profile defaults */
#define START_AUTO_HH   6
#define START_AUTO_MM   0
#define START_VEG_HH    6
#define START_VEG_MM    0
#define START_FLOR_HH   6
#define START_FLOR_MM   0

/* RTC fallback */
#define RTC_AUTO_SET_FROM_BUILD_TIME  1
