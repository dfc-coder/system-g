#include "app_state_storage.h"

#include <string.h>

#include "esp_err.h"
#include "esp_log.h"

#include "nvs.h"
#include "nvs_flash.h"

static const char *TAG = "app_state_store";

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

esp_err_t app_state_storage_load(state_t *io_state) {
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

esp_err_t app_state_storage_save(const state_t *state) {
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
