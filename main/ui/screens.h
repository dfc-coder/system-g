#pragma once

#include <stdbool.h>
#include <time.h>
#include "app_state.h"

typedef enum {
    SCREEN_STATUS = 0,
    SCREEN_MENU_ROOT,   // New root menu
    SCREEN_MENU_MODE,
    SCREEN_MENU_STAGE,
    SCREEN_CONFIG_TIME, // New time config screen
} screen_t;

void screen_render_status_simple(const state_t *st, const struct tm *now_tm, bool now_ok);
void screen_render_menu_root(int sel);
void screen_render_menu_mode(int sel);
void screen_render_menu_stage(int sel);
void screen_render_config_time(int hh, int mm, int focus_idx);