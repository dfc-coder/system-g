#include "freertos/FreeRTOS.h"

#include "esp_log.h"
#include "esp_err.h"

#include "nvs_flash.h"

#include "app_config.h"
#include "app_state.h"

#include "drivers/i2c_bus.h"
#include "drivers/oled_sh1106.h"
#include "drivers/relay.h"
#include "drivers/rtc_ds3231.h"
#include "drivers/encoder_pcnt.h"

#include "tasks/input_task.h"
#include "tasks/control_task.h"
#include "tasks/dht_task.h"
#include "ui/ui_task.h"

static const char *TAG = APP_TAG;

static void nvs_init_or_erase(void) {
    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ESP_ERROR_CHECK(nvs_flash_init());
        return;
    }
    ESP_ERROR_CHECK(err);
}

void app_main(void) {
    nvs_init_or_erase();
    app_state_init_defaults();

    relay_init();
    relay_set(false);

    ESP_ERROR_CHECK(i2c_bus_init_with_probe());
    ESP_ERROR_CHECK(oled_sh1106_init());

    esp_err_t err = rtc_ds3231_init();
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "DS3231 not available: %s", esp_err_to_name(err));
    } else {
        rtc_init_or_fix_time_and_seed_cycle();
    }

    ESP_ERROR_CHECK(encoder_pcnt_init());

    ui_task_start();
    input_task_start();
    control_task_start();
    dht_task_start();

    ESP_LOGI(TAG, "Started (modular, DS3231 schedule by clock + PCNT encoder + STATUS simplified)");
}
