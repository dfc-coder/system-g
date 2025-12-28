#pragma once

#include "esp_err.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

/* debe llamarse desde main */
void ui_task_start(void);

/* otros tasks llaman esto para forzar refresh */
void ui_task_notify_redraw(void);