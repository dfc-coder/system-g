#pragma once

#include <time.h>
#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"

esp_err_t rtc_ds3231_init(void);
bool rtc_time_now_local(struct tm *out);

esp_err_t rtc_ds3231_read_tm(struct tm *out);
esp_err_t rtc_ds3231_write_tm(const struct tm *in);

bool rtc_build_time_to_tm(struct tm *out);
bool rtc_tm_is_sane(const struct tm *t);

int  rtc_tm_minute_of_day(const struct tm *t);

/* AGE helpers */
int32_t rtc_daycount_from_tm(const struct tm *t);

/* init helper */
void rtc_init_or_fix_time_and_seed_cycle(void);
