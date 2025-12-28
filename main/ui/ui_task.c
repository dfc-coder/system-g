#include "ui/ui_task.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "esp_timer.h"
#include "esp_log.h"
#include "esp_err.h"

#include "app_config.h"
#include "app_state.h"

#include "drivers/oled_sh1106.h"
#include "drivers/i2c_bus.h"
#include "drivers/rtc_ds3231.h"

#include "tasks/input_task.h"

#include "ui/framebuffer.h"
#include "ui/screens.h"
#include "ui/ui_controller.h"

static const char *TAG = APP_TAG;
static TaskHandle_t g_ui_handle = NULL;

void ui_task_notify_redraw(void) {
    if (g_ui_handle) {
        xTaskNotifyGive(g_ui_handle);
    }
}

static void ui_flush_or_reset_bus(void) {
    esp_err_t e = oled_sh1106_flush_pages(
        fb_pages_flattened(),
        fb_pages_count(),
        fb_width()
    );

    if (e != ESP_OK) {
        ESP_LOGW(TAG, "OLED flush failed: %s (bus reset)", esp_err_to_name(e));
        i2c_bus_reset();
    }
}

static void wait_min_redraw(int64_t *last_draw_us) {
    int64_t now_us = esp_timer_get_time();
    if ((now_us - *last_draw_us) < (UI_MIN_REDRAW_MS * 1000LL)) {
        TickType_t tt = pdMS_TO_TICKS(UI_MIN_REDRAW_MS);
        if (tt == 0) tt = 1;
        vTaskDelay(tt);
    }
    *last_draw_us = esp_timer_get_time();
}

static void ui_task(void *arg) {
    (void)arg;

    g_ui_handle = xTaskGetCurrentTaskHandle();

    ui_controller_t controller;
    state_t st;
    struct tm now_tm = {0};
    int64_t last_draw_us = 0;

    bool ok = rtc_time_now_local(&now_tm);
    app_state_get_snapshot(&st);
    ui_controller_init(&controller, &st, ok ? &now_tm : NULL, ok);
    ui_controller_render(&controller, &st, ok ? &now_tm : NULL, ok);
    ui_flush_or_reset_bus();

    while (1) {
        uint32_t notified = ulTaskNotifyTake(pdTRUE, pdMS_TO_TICKS(500));

        wait_min_redraw(&last_draw_us);

        // Read current time and state for navigation decisions
        ok = rtc_time_now_local(&now_tm);
        app_state_get_snapshot(&st);

        int32_t delta_steps = 0;
        bool click = false;
        bool dirty_input = false;
        input_task_pop_events(&delta_steps, &click, &dirty_input);

        bool dirty = (notified > 0) || dirty_input;

        if (delta_steps != 0) {
            app_state_add_encoder_steps(delta_steps);
            app_state_set_last_evt((delta_steps > 0) ? '>' : '<');
            dirty |= ui_controller_on_encoder(&controller, delta_steps);
        }

        if (click) {
            app_state_set_last_evt('B');
            dirty |= ui_controller_on_click(&controller, &st, ok ? &now_tm : NULL, ok);
        }

        // Early exit if nothing to draw
        if (!dirty && delta_steps == 0 && !click) {
            continue;
        }

        // Refresh snapshot in case mode/stage changed inside controller
        app_state_get_snapshot(&st);
        ui_controller_render(&controller, &st, ok ? &now_tm : NULL, ok);
        ui_flush_or_reset_bus();
    }
}

void ui_task_start(void) {
    xTaskCreate(ui_task, "ui", 4096, NULL, PRIO_UI, NULL);
}
