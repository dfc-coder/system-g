#include "drivers/oled_sh1106.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "drivers/i2c_bus.h"
#include "app_config.h"

static esp_err_t oled_cmd(uint8_t c) {
    uint8_t b[2] = { 0x00, c };
    return i2c_bus_tx(i2c_bus_oled_dev(), b, sizeof(b), 400);
}

static esp_err_t oled_data_chunked(const uint8_t *data, size_t len) {
    uint8_t tmp[1 + 16];
    size_t off = 0;
    while (off < len) {
        size_t n = (len - off) > 16 ? 16 : (len - off);
        tmp[0] = 0x40;
        for (size_t i = 0; i < n; i++) tmp[1 + i] = data[off + i];
        esp_err_t err = i2c_bus_tx(i2c_bus_oled_dev(), tmp, 1 + n, 400);
        if (err != ESP_OK) return err;
        off += n;
    }
    return ESP_OK;
}

static void oled_set_page_col(uint8_t page, uint8_t col) {
    uint8_t c = (uint8_t)(col + 2); // offset típico SH1106
    (void)oled_cmd((uint8_t)(0xB0 | (page & 0x0F)));
    (void)oled_cmd((uint8_t)(0x10 | ((c >> 4) & 0x0F)));
    (void)oled_cmd((uint8_t)(0x00 | (c & 0x0F)));
}

esp_err_t oled_sh1106_init(void) {
    vTaskDelay(pdMS_TO_TICKS(120));

    esp_err_t err = ESP_OK;
    err = oled_cmd(0xAE); if (err) return err;
    err = oled_cmd(0xD5); if (err) return err; err = oled_cmd(0x80); if (err) return err;
    err = oled_cmd(0xA8); if (err) return err; err = oled_cmd(0x3F); if (err) return err;
    err = oled_cmd(0xD3); if (err) return err; err = oled_cmd(0x00); if (err) return err;
    err = oled_cmd(0x40); if (err) return err;
    err = oled_cmd(0xAD); if (err) return err; err = oled_cmd(0x8B); if (err) return err;
    err = oled_cmd(0xA1); if (err) return err;
    err = oled_cmd(0xC8); if (err) return err;
    err = oled_cmd(0xDA); if (err) return err; err = oled_cmd(0x12); if (err) return err;
    err = oled_cmd(0x81); if (err) return err; err = oled_cmd(0x7F); if (err) return err;
    err = oled_cmd(0xD9); if (err) return err; err = oled_cmd(0x22); if (err) return err;
    err = oled_cmd(0xDB); if (err) return err; err = oled_cmd(0x35); if (err) return err;
    err = oled_cmd(0xA4); if (err) return err;
    err = oled_cmd(0xA6); if (err) return err;
    err = oled_cmd(0xAF); if (err) return err;
    return ESP_OK;
}

esp_err_t oled_sh1106_flush_pages(const uint8_t *pages, int page_count, int width) {
    for (int p = 0; p < page_count; p++) {
        oled_set_page_col((uint8_t)p, 0);
        esp_err_t err = oled_data_chunked(&pages[p * width], (size_t)width);
        if (err != ESP_OK) return err;
    }
    return ESP_OK;
}
