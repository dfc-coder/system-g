#include "dht22.h"

#include "driver/gpio.h"
#include "esp_rom_sys.h"     // esp_rom_delay_us
#include "esp_timer.h"

static inline int64_t now_us(void) { return esp_timer_get_time(); }

static esp_err_t wait_level(int gpio_num, int level, int timeout_us) {
    int64_t t0 = now_us();

    while (gpio_get_level(gpio_num) != level) {
        if ((now_us() - t0) > timeout_us) return ESP_ERR_TIMEOUT;
        esp_rom_delay_us(1); // evita hog de CPU
    }
    return ESP_OK;
}

static esp_err_t wait_level_with_measure(int gpio_num, int level, int timeout_us, int *duration_us) {
    if (!duration_us) return ESP_ERR_INVALID_ARG;

    int64_t t0 = now_us();

    while (gpio_get_level(gpio_num) != level) {
        if ((now_us() - t0) > timeout_us) return ESP_ERR_TIMEOUT;
        esp_rom_delay_us(1); // evita hog de CPU
    }

    int64_t t1 = now_us();
    *duration_us = (int)(t1 - t0);
    return ESP_OK;
}

esp_err_t dht22_read(int gpio_num, dht22_reading_t *out) {
    if (!out) return ESP_ERR_INVALID_ARG;

    // 1) Start signal: pull low >= 1ms
    gpio_reset_pin(gpio_num);
    gpio_set_direction(gpio_num, GPIO_MODE_OUTPUT_OD);
    gpio_set_level(gpio_num, 0);
    esp_rom_delay_us(1200);

    // Release bus: high for 20-40us
    gpio_set_level(gpio_num, 1);
    esp_rom_delay_us(40);

    // 2) Switch to input and wait sensor response
    gpio_set_direction(gpio_num, GPIO_MODE_INPUT);
    gpio_set_pull_mode(gpio_num, GPIO_PULLUP_ONLY);

    // Sensor response: ~80us low, ~80us high, then data low
    esp_err_t err = wait_level(gpio_num, 0, 300);
    if (err != ESP_OK) return err;

    err = wait_level(gpio_num, 1, 300);
    if (err != ESP_OK) return err;

    err = wait_level(gpio_num, 0, 300);
    if (err != ESP_OK) return err;

    // 3) Read 40 bits:
    // each bit: ~50us low + high (26-28us=0, ~70us=1)
    uint8_t data[5] = {0};

    for (int i = 0; i < 40; i++) {
        // espera el HIGH (final del low de 50us)
        err = wait_level(gpio_num, 1, 150);
        if (err != ESP_OK) return err;

        // mide cuánto dura el HIGH hasta volver a LOW
        int high_us = 0;
        err = wait_level_with_measure(gpio_num, 0, 200, &high_us);
        if (err != ESP_OK) return err;

        int bit = (high_us > 50) ? 1 : 0;

        data[i / 8] <<= 1;
        data[i / 8] |= (uint8_t)bit;
    }

    // 4) checksum
    uint8_t sum = (uint8_t)(data[0] + data[1] + data[2] + data[3]);
    if (sum != data[4]) return ESP_ERR_INVALID_CRC;

    // 5) decode
    uint16_t hum_raw = (uint16_t)((data[0] << 8) | data[1]);
    uint16_t tmp_raw = (uint16_t)((data[2] << 8) | data[3]);

    float hum = hum_raw / 10.0f;

    bool neg = (tmp_raw & 0x8000) != 0;
    tmp_raw &= 0x7FFF;
    float temp = tmp_raw / 10.0f;
    if (neg) temp = -temp;

    out->humidity_pct = hum;
    out->temperature_c = temp;

    return ESP_OK;
}
