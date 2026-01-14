// main/app_state.c
#include "app_state.h"

#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include "esp_err.h"
#include "esp_log.h"

#include "app_config.h"
#include "app_state_logic.h"
#include "app_state_storage.h"
#include "drivers/rtc_ds3231.h"

static const char *TAG = "app_state";

static state_t g_state;
static SemaphoreHandle_t g_mtx = NULL;

static void ensure_mutex(void) {
    if (g_mtx) return;

    g_mtx = xSemaphoreCreateMutex();
    if (!g_mtx) {
        memset(&g_state, 0, sizeof(g_state));
    }
}

static bool lock_state(void) {
    ensure_mutex();
    if (!g_mtx) return false;
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    return true;
}

static void unlock_state(void) {
    if (g_mtx) {
        xSemaphoreGive(g_mtx);
    }
}

static void persist_snapshot(const state_t *snap, const char *context) {
    esp_err_t err = app_state_storage_save(snap);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Persist save failed (%s): %s", context, esp_err_to_name(err));
    }
}

void app_state_init_defaults(void) {
    if (!lock_state()) return;

    app_state_reset_defaults(&g_state);

    state_t loaded = g_state;
    esp_err_t err = app_state_storage_load(&loaded);
    if (err == ESP_OK) {
        app_state_apply_profile_schedule(&loaded);

        loaded.relay_on = false;
        loaded.time_valid = false;

        g_state = loaded;
        ESP_LOGI(TAG, "Loaded persisted state");
    } else {
        ESP_LOGW(TAG, "No persisted state, using defaults: %s", esp_err_to_name(err));
    }

    unlock_state();
}

void app_state_get_snapshot(state_t *out) {
    if (!out) return;

    if (!lock_state()) return;
    *out = g_state;
    unlock_state();
}

void app_state_set_time_valid(bool ok) {
    if (!lock_state()) return;
    g_state.time_valid = ok;
    unlock_state();
}

void app_state_set_relay_on(bool on) {
    if (!lock_state()) return;
    g_state.relay_on = on;
    unlock_state();
}

void app_state_add_encoder_steps(int32_t steps) {
    if (!lock_state()) return;
    g_state.enc_total_steps += steps;
    unlock_state();
}

void app_state_set_last_evt(char evt) {
    if (!lock_state()) return;
    g_state.last_evt = evt;
    unlock_state();
}

void app_state_set_mode(grow_mode_t mode, const struct tm *now_tm, bool now_ok) {
    state_t snap;

    if (!lock_state()) return;

    g_state.mode = mode;
    app_state_apply_profile_schedule(&g_state);

    // reset cycle start when changing mode (this is your current behavior)
    if (now_ok && now_tm) {
        g_state.cycle_start_set = true;
        g_state.cycle_start_daycount = rtc_daycount_from_tm(now_tm);
    } else {
        g_state.cycle_start_set = false;
        g_state.cycle_start_daycount = 0;
    }

    snap = g_state;
    unlock_state();

    // persist only on meaningful changes
    persist_snapshot(&snap, "mode");
}

void app_state_set_stage(stage_t stage, const struct tm *now_tm, bool now_ok) {
    state_t snap;

    if (!lock_state()) return;

    g_state.stage = stage;
    app_state_apply_profile_schedule(&g_state);

    // reset cycle start when changing stage (this is your current behavior)
    if (now_ok && now_tm) {
        g_state.cycle_start_set = true;
        g_state.cycle_start_daycount = rtc_daycount_from_tm(now_tm);
    } else {
        g_state.cycle_start_set = false;
        g_state.cycle_start_daycount = 0;
    }

    snap = g_state;
    unlock_state();

    persist_snapshot(&snap, "stage");
}

void app_state_set_dht_reading(bool ok, float temp_c, float hum_pct, int64_t ts_ms) {
    if (!lock_state()) return;
    g_state.dht_ok = ok;
    g_state.dht_temp_c = temp_c;
    g_state.dht_hum_pct = hum_pct;
    g_state.dht_last_read_ms = ts_ms;
    unlock_state();
}
