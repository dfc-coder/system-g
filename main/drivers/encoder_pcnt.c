#include "drivers/encoder_pcnt.h"

#include "esp_log.h"
#include "driver/gpio.h"
#include "driver/pulse_cnt.h"

#include "app_config.h"

static const char *TAG = APP_TAG;

static pcnt_unit_handle_t    g_unit = NULL;
static pcnt_channel_handle_t g_ch_a = NULL;
static pcnt_channel_handle_t g_ch_b = NULL;

static void encoder_gpio_init(void) {
    gpio_config_t io = {
        .pin_bit_mask = (1ULL << ENC_A_GPIO) | (1ULL << ENC_B_GPIO),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = ENC_AB_PULLUP ? 1 : 0,
        .pull_down_en = ENC_AB_PULLDOWN ? 1 : 0,
        .intr_type = GPIO_INTR_DISABLE,
    };
    ESP_ERROR_CHECK(gpio_config(&io));
}

static void pcnt_set_glitch_filter_best_effort(pcnt_unit_handle_t unit, uint32_t desired_ns) {
    if (desired_ns == 0) {
        pcnt_glitch_filter_config_t flt0 = { .max_glitch_ns = 0 };
        (void)pcnt_unit_set_glitch_filter(unit, &flt0);
        return;
    }

    uint32_t ns = desired_ns;
    esp_err_t err = ESP_ERR_INVALID_ARG;

    while (ns >= 50) {
        pcnt_glitch_filter_config_t flt = { .max_glitch_ns = ns };
        err = pcnt_unit_set_glitch_filter(unit, &flt);
        if (err == ESP_OK) {
            ESP_LOGI(TAG, "PCNT glitch filter set: %u ns", (unsigned)ns);
            return;
        }
        if (err != ESP_ERR_INVALID_ARG) break;
        ns /= 2;
    }

    ESP_LOGW(TAG, "PCNT glitch filter not set (requested %u ns): %s",
             (unsigned)desired_ns, esp_err_to_name(err));
}

esp_err_t encoder_pcnt_init(void) {
    encoder_gpio_init();

    pcnt_unit_config_t unit_cfg = {
        .low_limit = -32768,
        .high_limit = 32767,
        .flags.accum_count = 0,
    };
    ESP_ERROR_CHECK(pcnt_new_unit(&unit_cfg, &g_unit));

    pcnt_set_glitch_filter_best_effort(g_unit, ENC_GLITCH_NS);

    pcnt_chan_config_t ch_a_cfg = { .edge_gpio_num = ENC_A_GPIO, .level_gpio_num = ENC_B_GPIO };
    ESP_ERROR_CHECK(pcnt_new_channel(g_unit, &ch_a_cfg, &g_ch_a));

    pcnt_chan_config_t ch_b_cfg = { .edge_gpio_num = ENC_B_GPIO, .level_gpio_num = ENC_A_GPIO };
    ESP_ERROR_CHECK(pcnt_new_channel(g_unit, &ch_b_cfg, &g_ch_b));

    ESP_ERROR_CHECK(pcnt_channel_set_edge_action(g_ch_a,
        PCNT_CHANNEL_EDGE_ACTION_INCREASE, PCNT_CHANNEL_EDGE_ACTION_DECREASE));
    ESP_ERROR_CHECK(pcnt_channel_set_level_action(g_ch_a,
        PCNT_CHANNEL_LEVEL_ACTION_KEEP, PCNT_CHANNEL_LEVEL_ACTION_INVERSE));

    ESP_ERROR_CHECK(pcnt_channel_set_edge_action(g_ch_b,
        PCNT_CHANNEL_EDGE_ACTION_DECREASE, PCNT_CHANNEL_EDGE_ACTION_INCREASE));
    ESP_ERROR_CHECK(pcnt_channel_set_level_action(g_ch_b,
        PCNT_CHANNEL_LEVEL_ACTION_KEEP, PCNT_CHANNEL_LEVEL_ACTION_INVERSE));

    ESP_ERROR_CHECK(pcnt_unit_enable(g_unit));
    ESP_ERROR_CHECK(pcnt_unit_clear_count(g_unit));
    ESP_ERROR_CHECK(pcnt_unit_start(g_unit));
    return ESP_OK;
}

esp_err_t encoder_pcnt_get_and_clear(int *out_count) {
    if (!g_unit) return ESP_ERR_INVALID_STATE;
    if (!out_count) return ESP_ERR_INVALID_ARG;

    int c = 0;
    esp_err_t err = pcnt_unit_get_count(g_unit, &c);
    if (err != ESP_OK) return err;

    if (c != 0) (void)pcnt_unit_clear_count(g_unit);
    *out_count = c;
    return ESP_OK;
}
