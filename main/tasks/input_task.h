#pragma once
#include <stdbool.h>
#include <stdint.h>

void input_task_start(void);

/* UI consume eventos y los limpia */
void input_task_pop_events(int32_t *delta_steps, bool *btn_click, bool *dirty);
