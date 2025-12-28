#pragma once
#include <stdint.h>
#include "esp_err.h"

typedef struct {
    float temperature_c;
    float humidity_pct;
} dht22_reading_t;

// Lee DHT22 (AM2302). Devuelve ESP_OK si checksum y timing OK.
esp_err_t dht22_read(int gpio_num, dht22_reading_t *out);
