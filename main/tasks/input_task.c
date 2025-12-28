#include "tasks/input_task.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "esp_err.h"

#include "iot_button.h"
#include "button_gpio.h"

#include "app_config.h"
#include "drivers/encoder_pcnt.h"
#include "ui/ui_task.h"

static portMUX_TYPE g_mux = portMUX_INITIALIZER_UNLOCKED;
static volatile int32_t g_enc_delta = 0;
static volatile bool    g_btn_click = false;
static volatile bool    g_ui_dirty  = false;

static button_handle_t btn_handle = NULL;

static inline int iabs_int(int v) { return (v < 0) ? -v : v; }

void input_task_pop_events(int32_t *delta_steps, bool *btn_click, bool *dirty) {
    if (!delta_steps || !btn_click || !dirty) return;

    portENTER_CRITICAL(&g_mux);
    *delta_steps = g_enc_delta; g_enc_delta = 0;
    *btn_click   = g_btn_click; g_btn_click = false;
    *dirty       = g_ui_dirty;  g_ui_dirty = false;
    portEXIT_CRITICAL(&g_mux);
}

// Button callback - runs in iot_button's internal task context
static void button_release_cb(void *button_handle, void *usr_data) {
    (void)button_handle;
    (void)usr_data;

    ESP_LOGD("input_task", "Button released");

    portENTER_CRITICAL(&g_mux);
    g_btn_click = true;
    g_ui_dirty = true;
    portEXIT_CRITICAL(&g_mux);

    ui_task_notify_redraw();
}

static void input_task(void *arg) {
    (void)arg;

    int32_t edges_accum = 0;
    TickType_t delay_ticks = pdMS_TO_TICKS(INPUT_TASK_MS);
    if (delay_ticks == 0) delay_ticks = 1;

    while (1) {
        vTaskDelay(delay_ticks);

        int count = 0;
        if (encoder_pcnt_get_and_clear(&count) == ESP_OK && count != 0) {
            if (ENC_DIR_INVERT) count = -count;

            if (iabs_int(count) > ENC_MAX_EDGES_PER_SAMPLE) {
                edges_accum = 0;
            } else {
                edges_accum += count;

                int32_t steps = edges_accum / ENC_EDGES_PER_STEP;
                edges_accum -= steps * ENC_EDGES_PER_STEP;

                if (steps != 0) {
                    portENTER_CRITICAL(&g_mux);
                    g_enc_delta += steps;
                    g_ui_dirty = true;
                    portEXIT_CRITICAL(&g_mux);
                    ui_task_notify_redraw();
                }
            }
        }
    }
}

void input_task_start(void) {
    button_config_t btn_cfg = {
        .long_press_time = 1000,
        .short_press_time = BTN_DEBOUNCE_MS,
    };

    button_gpio_config_t gpio_cfg = {
        .gpio_num = ENC_BTN_GPIO,
        .active_level = 0,
        .enable_power_save = false,
        .disable_pull = false,
    };

    esp_err_t ret = iot_button_new_gpio_device(&btn_cfg, &gpio_cfg, &btn_handle);
    if (ret != ESP_OK || btn_handle == NULL) {
        ESP_LOGE("input_task", "Button creation failed: %s", esp_err_to_name(ret));
    } else {
        // ✅ NUEVA FIRMA: 5 args (event_args va 3°)
        ret = iot_button_register_cb(btn_handle, BUTTON_PRESS_UP, NULL, button_release_cb, NULL);
        if (ret != ESP_OK) {
            ESP_LOGE("input_task", "Button callback registration failed: %s", esp_err_to_name(ret));
        }
    }

    xTaskCreate(input_task, "input", 3072, NULL, PRIO_INPUT, NULL);
}
