// main/tasks/dht_task.c
#include "tasks/dht_task.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "esp_err.h"
#include "esp_log.h"
#include "esp_timer.h"

#include "drivers/dht22.h"
#include "app_state.h"
#include "ui/ui_task.h"

#ifndef DHT_GPIO
#define DHT_GPIO 27
#endif

#ifndef DHT_POLL_MS
#define DHT_POLL_MS 5000
#endif

#ifndef DHT_TASK_STACK
#define DHT_TASK_STACK 3072
#endif

#ifndef DHT_TASK_PRIO
#define DHT_TASK_PRIO 2
#endif

static const char *TAG = "dht_task";
static TaskHandle_t s_dht_task = NULL;

static inline int64_t now_ms(void) {
    return esp_timer_get_time() / 1000;
}

static void dht_task_run(void *arg) {
    (void)arg;

    vTaskDelay(pdMS_TO_TICKS(250));

    while (1) {
        dht22_reading_t r = {0};
        esp_err_t err = dht22_read(DHT_GPIO, &r);

        if (err == ESP_OK) {
            app_state_set_dht_reading(true, r.temperature_c, r.humidity_pct, now_ms());
        } else {
            app_state_set_dht_reading(false, 0.0f, 0.0f, now_ms());
            ESP_LOGW(TAG, "read failed: %s", esp_err_to_name(err));
        }

        ui_task_notify_redraw();

        vTaskDelay(pdMS_TO_TICKS(DHT_POLL_MS));
    }
}

void dht_task_start(void) {
    if (s_dht_task) return;

    BaseType_t ok = xTaskCreate(
        dht_task_run,
        "dht",
        DHT_TASK_STACK,
        NULL,
        DHT_TASK_PRIO,
        &s_dht_task
    );

    if (ok != pdPASS) {
        s_dht_task = NULL;
        ESP_LOGE(TAG, "xTaskCreate failed");
    }
}
