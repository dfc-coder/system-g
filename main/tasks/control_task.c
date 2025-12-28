#include "tasks/control_task.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "esp_log.h"

#include "app_config.h"
#include "app_state.h"

#include "drivers/relay.h"
#include "drivers/rtc_ds3231.h"

#include "ui/ui_task.h"

static const char *TAG = APP_TAG;

static void control_task(void *arg) {
    (void)arg;

    TickType_t t = pdMS_TO_TICKS(CONTROL_TASK_MS);
    if (t == 0) t = 1;

    int last_minute_seen = -1;

    while (1) {
        vTaskDelay(t);

        struct tm now_tm = {0};
        bool ok = rtc_time_now_local(&now_tm);

        app_state_set_time_valid(ok);
        if (!ok) {
            // failsafe OFF
            state_t st;
            app_state_get_snapshot(&st);
            if (st.relay_on) {
                relay_set(false);
                app_state_set_relay_on(false);
                ui_task_notify_redraw();
            }
            continue;
        }

        int now_min = rtc_tm_minute_of_day(&now_tm);
        if (now_min != last_minute_seen) {
            last_minute_seen = now_min;
            ui_task_notify_redraw();
        }

        state_t st;
        app_state_get_snapshot(&st);

        bool should_on = app_state_schedule_should_on(now_min, st.start_min_of_day, st.on_min_per_day);
        if (should_on != st.relay_on) {
            relay_set(should_on);
            app_state_set_relay_on(should_on);
            ui_task_notify_redraw();
            ESP_LOGI(TAG, "Relay -> %s (%02d:%02d)", should_on ? "ON" : "OFF", now_tm.tm_hour, now_tm.tm_min);
        }
    }
}

void control_task_start(void) {
    xTaskCreate(control_task, "control", 3072, NULL, PRIO_CTL, NULL);
}
