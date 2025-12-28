#pragma once
#include "esp_err.h"

esp_err_t encoder_pcnt_init(void);

/* Lee el count acumulado y lo limpia */
esp_err_t encoder_pcnt_get_and_clear(int *out_count);
