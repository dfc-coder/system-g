#include "drivers/rtc_ds3231.h"

#include <string.h>

#include "app_config.h"
#include "drivers/i2c_bus.h"
#include "app_state.h"

static inline uint8_t bcd2bin(uint8_t v) { return (uint8_t)((v >> 4) * 10 + (v & 0x0F)); }
static inline uint8_t bin2bcd(uint8_t v) { return (uint8_t)(((v / 10) << 4) | (v % 10)); }

static i2c_master_dev_handle_t g_rtc = NULL;

esp_err_t rtc_ds3231_init(void) {
    g_rtc = i2c_bus_rtc_dev();
    return (g_rtc != NULL) ? ESP_OK : ESP_ERR_NOT_FOUND;
}

bool rtc_tm_is_sane(const struct tm *t) {
    if (!t) return false;
    if (t->tm_year < (2023 - 1900) || t->tm_year > (2099 - 1900)) return false;
    if (t->tm_mon < 0 || t->tm_mon > 11) return false;
    if (t->tm_mday < 1 || t->tm_mday > 31) return false;
    if (t->tm_hour < 0 || t->tm_hour > 23) return false;
    if (t->tm_min < 0 || t->tm_min > 59) return false;
    if (t->tm_sec < 0 || t->tm_sec > 59) return false;
    return true;
}

esp_err_t rtc_ds3231_read_tm(struct tm *out) {
    if (!g_rtc) return ESP_ERR_INVALID_STATE;
    if (!out) return ESP_ERR_INVALID_ARG;

    uint8_t reg = 0x00;
    uint8_t buf[7] = {0};

    esp_err_t err = i2c_bus_txrx(g_rtc, &reg, 1, buf, sizeof(buf), 400);
    if (err != ESP_OK) return err;

    int sec = bcd2bin(buf[0] & 0x7F);
    int min = bcd2bin(buf[1] & 0x7F);

    uint8_t hr_raw = buf[2];
    int hour = 0;
    if (hr_raw & 0x40) { // 12h
        int h12 = bcd2bin(hr_raw & 0x1F);
        bool pm = (hr_raw & 0x20) != 0;
        if (h12 == 12) hour = pm ? 12 : 0;
        else hour = pm ? (h12 + 12) : h12;
    } else {
        hour = bcd2bin(hr_raw & 0x3F);
    }

    int mday = bcd2bin(buf[4] & 0x3F);
    int mon  = bcd2bin(buf[5] & 0x1F);
    int year = bcd2bin(buf[6]);

    struct tm t = {0};
    t.tm_sec  = sec;
    t.tm_min  = min;
    t.tm_hour = hour;
    t.tm_mday = mday;
    t.tm_mon  = mon - 1;
    t.tm_year = (2000 + year) - 1900;

    if (!rtc_tm_is_sane(&t)) return ESP_ERR_INVALID_RESPONSE;

    *out = t;
    return ESP_OK;
}

esp_err_t rtc_ds3231_write_tm(const struct tm *in) {
    if (!g_rtc) return ESP_ERR_INVALID_STATE;
    if (!in) return ESP_ERR_INVALID_ARG;
    if (!rtc_tm_is_sane(in)) return ESP_ERR_INVALID_ARG;

    uint8_t w[1 + 7] = {0};
    w[0] = 0x00;
    w[1] = bin2bcd((uint8_t)in->tm_sec);
    w[2] = bin2bcd((uint8_t)in->tm_min);
    w[3] = bin2bcd((uint8_t)in->tm_hour); // 24h
    w[4] = bin2bcd(1);                    // DOW dummy
    w[5] = bin2bcd((uint8_t)in->tm_mday);
    w[6] = bin2bcd((uint8_t)(in->tm_mon + 1));
    w[7] = bin2bcd((uint8_t)((in->tm_year + 1900) - 2000));

    return i2c_bus_tx(g_rtc, w, sizeof(w), 400);
}

static int month_from_str3(const char *m) {
    if (!m) return -1;
    if (memcmp(m, "Jan", 3) == 0) return 0;
    if (memcmp(m, "Feb", 3) == 0) return 1;
    if (memcmp(m, "Mar", 3) == 0) return 2;
    if (memcmp(m, "Apr", 3) == 0) return 3;
    if (memcmp(m, "May", 3) == 0) return 4;
    if (memcmp(m, "Jun", 3) == 0) return 5;
    if (memcmp(m, "Jul", 3) == 0) return 6;
    if (memcmp(m, "Aug", 3) == 0) return 7;
    if (memcmp(m, "Sep", 3) == 0) return 8;
    if (memcmp(m, "Oct", 3) == 0) return 9;
    if (memcmp(m, "Nov", 3) == 0) return 10;
    if (memcmp(m, "Dec", 3) == 0) return 11;
    return -1;
}

bool rtc_build_time_to_tm(struct tm *out) {
    if (!out) return false;

    const char *d = __DATE__; // "Mmm dd yyyy"
    const char *t = __TIME__; // "hh:mm:ss"

    int mon = month_from_str3(d);
    if (mon < 0) return false;

    int day = (d[4] == ' ') ? (d[5] - '0') : ((d[4] - '0') * 10 + (d[5] - '0'));
    int year = (d[7] - '0') * 1000 + (d[8] - '0') * 100 + (d[9] - '0') * 10 + (d[10] - '0');

    int hh = (t[0] - '0') * 10 + (t[1] - '0');
    int mm = (t[3] - '0') * 10 + (t[4] - '0');
    int ss = (t[6] - '0') * 10 + (t[7] - '0');

    struct tm tmv = {0};
    tmv.tm_year = year - 1900;
    tmv.tm_mon  = mon;
    tmv.tm_mday = day;
    tmv.tm_hour = hh;
    tmv.tm_min  = mm;
    tmv.tm_sec  = ss;

    if (!rtc_tm_is_sane(&tmv)) return false;
    *out = tmv;
    return true;
}

bool rtc_time_now_local(struct tm *out) {
    if (!out) return false;
    if (!g_rtc) return false;
    return (rtc_ds3231_read_tm(out) == ESP_OK);
}

int rtc_tm_minute_of_day(const struct tm *t) {
    return (t->tm_hour * 60) + t->tm_min;
}

/* days since 1970-01-01 */
static int32_t civil_to_days_int(int y, unsigned m, unsigned d) {
    y -= m <= 2;
    const int era = (y >= 0 ? y : y - 399) / 400;
    const unsigned yoe = (unsigned)(y - era * 400);
    const unsigned doy = (153 * (m + (m > 2 ? (unsigned)-3 : 9)) + 2) / 5 + d - 1;
    const unsigned doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return (int32_t)(era * 146097 + (int)doe - 719468);
}

int32_t rtc_daycount_from_tm(const struct tm *t) {
    int y = t->tm_year + 1900;
    unsigned m = (unsigned)(t->tm_mon + 1);
    unsigned d = (unsigned)t->tm_mday;
    return civil_to_days_int(y, m, d);
}

void rtc_init_or_fix_time_and_seed_cycle(void) {
    if (!g_rtc) {
        app_state_set_time_valid(false);
        return;
    }

    struct tm t = {0};
    esp_err_t err = rtc_ds3231_read_tm(&t);
    if (err == ESP_OK) {
        app_state_set_time_valid(true);
        // si el ciclo no estaba seteado, queda con AGE=0 desde hoy (se setea en app_state al cambiar perfil)
        return;
    }

#if RTC_AUTO_SET_FROM_BUILD_TIME
    struct tm bt = {0};
    if (rtc_build_time_to_tm(&bt) && rtc_ds3231_write_tm(&bt) == ESP_OK) {
        app_state_set_time_valid(true);
        return;
    }
#endif

    app_state_set_time_valid(false);
}
