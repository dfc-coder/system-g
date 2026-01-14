#pragma once

#include <stdbool.h>
#include <time.h>

#include "app_state.h"
#include "ui/screens.h"

typedef struct {
    screen_t screen;
    int sel_mode;
    int sel_stage;
    int sel_root;

    int cfg_hh;
    int cfg_mm;
    int cfg_focus; // 0 = HH, 1 = MM
} ui_controller_t;

void ui_controller_init(ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok);

// Returns true when the input changed UI state and needs redraw.
bool ui_controller_on_encoder(ui_controller_t *ctrl, int32_t steps);
bool ui_controller_on_click(ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok);

// Renders the current screen.
void ui_controller_render(const ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok);
