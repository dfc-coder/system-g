// main/app_state.c
#include "app_state.h"

#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include "esp_err.h"
#include "esp_log.h"

#include "nvs.h"
#include "nvs_flash.h"

#include "app_config.h"
#include "drivers/rtc_ds3231.h"

static const char *TAG = "app_state";

static state_t g_state;
static SemaphoreHandle_t g_mtx = NULL;

// ==========================================================
// NVS persistence (to keep AGE + profile across reboots)
// ==========================================================

#define APP_STATE_NVS_NS   "grow"
#define APP_STATE_NVS_KEY  "state_v1"
#define APP_STATE_MAGIC    0x47524F57u  // 'GROW'
#define APP_STATE_VERSION  1u

typedef struct {
    uint32_t magic;
    uint16_t version;

    uint8_t  mode;   // grow_mode_t
    uint8_t  stage;  // stage_t

    uint16_t start_min_of_day;
    uint16_t on_min_per_day;

    uint8_t  cycle_start_set;
    int32_t  cycle_start_daycount;

    uint32_t crc;
} app_state_blob_v1_t;

static uint32_t fnv1a32(const void *data, size_t len) {
    const uint8_t *p = (const uint8_t *)data;
    uint32_t h = 2166136261u;
    for (size_t i = 0; i < len; i++) {
        h ^= p[i];
        h *= 16777619u;
    }
    return h;
}

static esp_err_t ensure_nvs_ready(void) {
    static bool s_ready = false;
    if (s_ready) return ESP_OK;

    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_LOGW(TAG, "NVS init requires erase: %s", esp_err_to_name(err));
        ESP_ERROR_CHECK(nvs_flash_erase());
        err = nvs_flash_init();
    }

    if (err == ESP_OK) s_ready = true;
    return err;
}

static app_state_blob_v1_t pack_blob_from_state(const state_t *st) {
    app_state_blob_v1_t b;
    memset(&b, 0, sizeof(b));

    b.magic = APP_STATE_MAGIC;
    b.version = APP_STATE_VERSION;

    b.mode = (uint8_t)st->mode;
    b.stage = (uint8_t)st->stage;

    b.start_min_of_day = st->start_min_of_day;
    b.on_min_per_day   = st->on_min_per_day;

    b.cycle_start_set = st->cycle_start_set ? 1 : 0;
    b.cycle_start_daycount = st->cycle_start_daycount;

    b.crc = 0;
    b.crc = fnv1a32(&b, sizeof(b));
    return b;
}

static bool unpack_blob_to_state(const app_state_blob_v1_t *b, state_t *st) {
    if (!b || !st) return false;
    if (b->magic != APP_STATE_MAGIC) return false;
    if (b->version != APP_STATE_VERSION) return false;

    app_state_blob_v1_t tmp = *b;
    uint32_t saved = tmp.crc;
    tmp.crc = 0;
    tmp.crc = fnv1a32(&tmp, sizeof(tmp));
    if (tmp.crc != saved) return false;

    st->mode = (grow_mode_t)b->mode;
    st->stage = (stage_t)b->stage;

    st->start_min_of_day = b->start_min_of_day;
    st->on_min_per_day   = b->on_min_per_day;

    st->cycle_start_set = (b->cycle_start_set != 0);
    st->cycle_start_daycount = b->cycle_start_daycount;

    return true;
}

static esp_err_t load_from_nvs(state_t *io_state) {
    if (!io_state) return ESP_ERR_INVALID_ARG;

    esp_err_t err = ensure_nvs_ready();
    if (err != ESP_OK) return err;

    nvs_handle_t h;
    err = nvs_open(APP_STATE_NVS_NS, NVS_READONLY, &h);
    if (err != ESP_OK) return err;

    app_state_blob_v1_t b;
    size_t sz = sizeof(b);
    err = nvs_get_blob(h, APP_STATE_NVS_KEY, &b, &sz);
    nvs_close(h);

    if (err != ESP_OK) return err;
    if (sz != sizeof(b)) return ESP_ERR_INVALID_SIZE;

    if (!unpack_blob_to_state(&b, io_state)) return ESP_ERR_INVALID_CRC;
    return ESP_OK;
}

static esp_err_t save_to_nvs(const state_t *state) {
    if (!state) return ESP_ERR_INVALID_ARG;

    esp_err_t err = ensure_nvs_ready();
    if (err != ESP_OK) return err;

    app_state_blob_v1_t b = pack_blob_from_state(state);

    nvs_handle_t h;
    err = nvs_open(APP_STATE_NVS_NS, NVS_READWRITE, &h);
    if (err != ESP_OK) return err;

    err = nvs_set_blob(h, APP_STATE_NVS_KEY, &b, sizeof(b));
    if (err == ESP_OK) err = nvs_commit(h);
    nvs_close(h);

    return err;
}

// ==========================================================
// Schedule + defaults helpers
// ==========================================================

uint16_t app_state_hhmm_to_min(uint8_t hh, uint8_t mm) {
    return (uint16_t)((hh % 24) * 60 + (mm % 60));
}

bool app_state_schedule_should_on(int now_min, uint16_t start_min, uint16_t on_min) {
    if (on_min == 0) return false;
    if (on_min >= 1440) return true;

    int end = (start_min + on_min) % 1440;

    if ((int)start_min < end) {
        return (now_min >= (int)start_min) && (now_min < end);
    }
    return (now_min >= (int)start_min) || (now_min < end);
}

static void apply_profile_schedule(state_t *st) {
    if (!st) return;

    if (st->mode == GROW_MODE_AUTO) {
        st->start_min_of_day = app_state_hhmm_to_min(START_AUTO_HH, START_AUTO_MM);
        st->on_min_per_day   = 20 * 60;
        return;
    }

    if (st->stage == STAGE_FLOR) {
        st->start_min_of_day = app_state_hhmm_to_min(START_FLOR_HH, START_FLOR_MM);
        st->on_min_per_day   = 12 * 60;
        return;
    }

    st->start_min_of_day = app_state_hhmm_to_min(START_VEG_HH, START_VEG_MM);
    st->on_min_per_day   = 18 * 60;
}

static void set_defaults(state_t *st) {
    memset(st, 0, sizeof(*st));

    st->mode = GROW_MODE_AUTO;
    st->stage = STAGE_VEG;

    st->relay_on = false;
    st->time_valid = false;

    st->cycle_start_set = false;
    st->cycle_start_daycount = 0;

    st->enc_total_steps = 0;
    st->last_evt = '-';

    st->dht_ok = false;
    st->dht_temp_c = 0.0f;
    st->dht_hum_pct = 0.0f;
    st->dht_last_read_ms = 0;

    apply_profile_schedule(st);
}

// ==========================================================
// Public API
// ==========================================================

void app_state_init_defaults(void) {
    if (!g_mtx) {
        g_mtx = xSemaphoreCreateMutex();
        if (!g_mtx) {
            memset(&g_state, 0, sizeof(g_state));
            return;
        }
    }

    xSemaphoreTake(g_mtx, portMAX_DELAY);

    set_defaults(&g_state);

    // Try load persisted
    state_t loaded = g_state;
    esp_err_t err = load_from_nvs(&loaded);
    if (err == ESP_OK) {
        // keep schedule consistent with loaded profile (AUTO/PHOTO stage)
        apply_profile_schedule(&loaded);

        // do not trust persisted relay/time health
        loaded.relay_on = false;
        loaded.time_valid = false;

        g_state = loaded;
        ESP_LOGI(TAG, "Loaded persisted state");
    } else {
        ESP_LOGW(TAG, "No persisted state, using defaults: %s", esp_err_to_name(err));
    }

    xSemaphoreGive(g_mtx);
}

void app_state_get_snapshot(state_t *out) {
    if (!out) return;

    xSemaphoreTake(g_mtx, portMAX_DELAY);
    *out = g_state;
    xSemaphoreGive(g_mtx);
}

void app_state_set_time_valid(bool ok) {
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    g_state.time_valid = ok;
    xSemaphoreGive(g_mtx);
}

void app_state_set_relay_on(bool on) {
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    g_state.relay_on = on;
    xSemaphoreGive(g_mtx);
}

void app_state_add_encoder_steps(int32_t steps) {
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    g_state.enc_total_steps += steps;
    xSemaphoreGive(g_mtx);
}

void app_state_set_last_evt(char evt) {
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    g_state.last_evt = evt;
    xSemaphoreGive(g_mtx);
}

void app_state_set_mode(grow_mode_t mode, const struct tm *now_tm, bool now_ok) {
    state_t snap;

    xSemaphoreTake(g_mtx, portMAX_DELAY);

    g_state.mode = mode;
    apply_profile_schedule(&g_state);

    // reset cycle start when changing mode (this is your current behavior)
    if (now_ok && now_tm) {
        g_state.cycle_start_set = true;
        g_state.cycle_start_daycount = rtc_daycount_from_tm(now_tm);
    } else {
        g_state.cycle_start_set = false;
        g_state.cycle_start_daycount = 0;
    }

    snap = g_state;
    xSemaphoreGive(g_mtx);

    // persist only on meaningful changes
    esp_err_t err = save_to_nvs(&snap);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Persist save failed (mode): %s", esp_err_to_name(err));
    }
}

void app_state_set_stage(stage_t stage, const struct tm *now_tm, bool now_ok) {
    state_t snap;

    xSemaphoreTake(g_mtx, portMAX_DELAY);

    g_state.stage = stage;
    apply_profile_schedule(&g_state);

    // reset cycle start when changing stage (this is your current behavior)
    if (now_ok && now_tm) {
        g_state.cycle_start_set = true;
        g_state.cycle_start_daycount = rtc_daycount_from_tm(now_tm);
    } else {
        g_state.cycle_start_set = false;
        g_state.cycle_start_daycount = 0;
    }

    snap = g_state;
    xSemaphoreGive(g_mtx);

    esp_err_t err = save_to_nvs(&snap);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Persist save failed (stage): %s", esp_err_to_name(err));
    }
}

void app_state_set_dht_reading(bool ok, float temp_c, float hum_pct, int64_t ts_ms) {
    xSemaphoreTake(g_mtx, portMAX_DELAY);
    g_state.dht_ok = ok;
    g_state.dht_temp_c = temp_c;
    g_state.dht_hum_pct = hum_pct;
    g_state.dht_last_read_ms = ts_ms;
    xSemaphoreGive(g_mtx);
}
