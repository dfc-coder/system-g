#pragma once

#include "esp_err.h"
#include <stdint.h>

esp_err_t oled_sh1106_init(void);
esp_err_t oled_sh1106_flush_pages(const uint8_t *pages, int page_count, int width);
