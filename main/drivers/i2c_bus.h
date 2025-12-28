#pragma once

#include "esp_err.h"
#include "driver/i2c_master.h"

esp_err_t i2c_bus_init_with_probe(void);

i2c_master_bus_handle_t i2c_bus_handle(void);
i2c_master_dev_handle_t i2c_bus_oled_dev(void);
i2c_master_dev_handle_t i2c_bus_rtc_dev(void);

esp_err_t i2c_bus_tx(i2c_master_dev_handle_t dev, const uint8_t *data, size_t len, int timeout_ms);
esp_err_t i2c_bus_txrx(i2c_master_dev_handle_t dev, const uint8_t *tx, size_t tx_len,
                       uint8_t *rx, size_t rx_len, int timeout_ms);

void i2c_bus_reset(void);
