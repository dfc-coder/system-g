#include "drivers/i2c_bus.h"

#include <string.h>

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"

#include "esp_log.h"
#include "esp_rom_sys.h"

#include "driver/gpio.h"
#include "app_config.h"

static const char *TAG = APP_TAG;

static SemaphoreHandle_t g_i2c_mtx;

static i2c_master_bus_handle_t g_bus = NULL;
static i2c_master_dev_handle_t g_oled = NULL;
static i2c_master_dev_handle_t g_rtc  = NULL;

static void i2c_gpio_bus_recover(void) {
    gpio_reset_pin(I2C_SCL_GPIO);
    gpio_reset_pin(I2C_SDA_GPIO);

    gpio_set_direction(I2C_SCL_GPIO, GPIO_MODE_INPUT_OUTPUT_OD);
    gpio_set_direction(I2C_SDA_GPIO, GPIO_MODE_INPUT_OUTPUT_OD);

    gpio_set_pull_mode(I2C_SCL_GPIO, GPIO_PULLUP_ONLY);
    gpio_set_pull_mode(I2C_SDA_GPIO, GPIO_PULLUP_ONLY);

    gpio_set_level(I2C_SCL_GPIO, 1);
    gpio_set_level(I2C_SDA_GPIO, 1);
    esp_rom_delay_us(10);

    for (int i = 0; i < 12; i++) {
        if (gpio_get_level(I2C_SDA_GPIO) == 1) break;
        gpio_set_level(I2C_SCL_GPIO, 0); esp_rom_delay_us(5);
        gpio_set_level(I2C_SCL_GPIO, 1); esp_rom_delay_us(5);
    }

    // STOP
    gpio_set_level(I2C_SDA_GPIO, 0); esp_rom_delay_us(5);
    gpio_set_level(I2C_SCL_GPIO, 1); esp_rom_delay_us(5);
    gpio_set_level(I2C_SDA_GPIO, 1); esp_rom_delay_us(5);

    gpio_reset_pin(I2C_SCL_GPIO);
    gpio_reset_pin(I2C_SDA_GPIO);
}

esp_err_t i2c_bus_init_with_probe(void) {
    g_i2c_mtx = xSemaphoreCreateMutex();

    i2c_gpio_bus_recover();

    i2c_master_bus_config_t bus_cfg = {
        .i2c_port = I2C_PORT,
        .sda_io_num = I2C_SDA_GPIO,
        .scl_io_num = I2C_SCL_GPIO,
        .clk_source = I2C_CLK_SRC_DEFAULT,
        .glitch_ignore_cnt = 7,
        .intr_priority = 0,
        .flags.enable_internal_pullup = true,
    };

    esp_err_t err = i2c_new_master_bus(&bus_cfg, &g_bus);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "i2c_new_master_bus failed: %s", esp_err_to_name(err));
        return err;
    }

    // OLED probe (reintentos)
    err = ESP_FAIL;
    for (int attempt = 1; attempt <= 8; attempt++) {
        err = i2c_master_probe(g_bus, OLED_ADDR, 250);
        if (err == ESP_OK) {
            ESP_LOGI(TAG, "OLED found at 0x%02X (attempt %d)", OLED_ADDR, attempt);
            break;
        }
        ESP_LOGW(TAG, "OLED probe failed (attempt %d): %s", attempt, esp_err_to_name(err));
        (void)i2c_master_bus_reset(g_bus);
        vTaskDelay(pdMS_TO_TICKS(80));
    }
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "OLED not found: %s", esp_err_to_name(err));
        return err;
    }

    i2c_device_config_t oled_cfg = {
        .dev_addr_length = I2C_ADDR_BIT_LEN_7,
        .device_address = OLED_ADDR,
        .scl_speed_hz = I2C_FREQ_HZ,
    };
    err = i2c_master_bus_add_device(g_bus, &oled_cfg, &g_oled);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "add OLED device failed: %s", esp_err_to_name(err));
        return err;
    }

    // RTC probe (opcional)
    err = i2c_master_probe(g_bus, RTC_ADDR, 250);
    if (err == ESP_OK) {
        ESP_LOGI(TAG, "DS3231 found at 0x%02X", RTC_ADDR);

        i2c_device_config_t rtc_cfg = {
            .dev_addr_length = I2C_ADDR_BIT_LEN_7,
            .device_address = RTC_ADDR,
            .scl_speed_hz = I2C_FREQ_HZ,
        };
        err = i2c_master_bus_add_device(g_bus, &rtc_cfg, &g_rtc);
        if (err != ESP_OK) {
            ESP_LOGW(TAG, "add RTC device failed: %s (RTC disabled)", esp_err_to_name(err));
            g_rtc = NULL;
        }
    } else {
        ESP_LOGW(TAG, "DS3231 probe failed: %s (RTC disabled)", esp_err_to_name(err));
        g_rtc = NULL;
    }

    return ESP_OK;
}

i2c_master_bus_handle_t i2c_bus_handle(void) { return g_bus; }
i2c_master_dev_handle_t i2c_bus_oled_dev(void) { return g_oled; }
i2c_master_dev_handle_t i2c_bus_rtc_dev(void) { return g_rtc; }

esp_err_t i2c_bus_tx(i2c_master_dev_handle_t dev, const uint8_t *data, size_t len, int timeout_ms) {
    if (!dev) return ESP_ERR_INVALID_STATE;
    xSemaphoreTake(g_i2c_mtx, portMAX_DELAY);
    esp_err_t err = i2c_master_transmit(dev, data, len, timeout_ms);
    xSemaphoreGive(g_i2c_mtx);
    return err;
}

esp_err_t i2c_bus_txrx(i2c_master_dev_handle_t dev, const uint8_t *tx, size_t tx_len,
                       uint8_t *rx, size_t rx_len, int timeout_ms) {
    if (!dev) return ESP_ERR_INVALID_STATE;
    xSemaphoreTake(g_i2c_mtx, portMAX_DELAY);
    esp_err_t err = i2c_master_transmit_receive(dev, tx, tx_len, rx, rx_len, timeout_ms);
    xSemaphoreGive(g_i2c_mtx);
    return err;
}

void i2c_bus_reset(void) {
    if (!g_bus) return;
    (void)i2c_master_bus_reset(g_bus);
}
