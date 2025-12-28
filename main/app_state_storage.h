#pragma once

#include "app_state.h"
#include "esp_err.h"

esp_err_t app_state_storage_load(state_t *io_state);
esp_err_t app_state_storage_save(const state_t *state);
