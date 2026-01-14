#include "ui/ui_controller.h"

#include "drivers/rtc_ds3231.h"

void ui_controller_init(ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok) {
    if (!ctrl) return;

    ctrl->screen = SCREEN_STATUS;
    ctrl->sel_mode = (state && state->mode == GROW_MODE_PHOTO) ? 1 : 0;
    ctrl->sel_stage = (state && state->stage == STAGE_FLOR) ? 1 : 0;
    ctrl->sel_root = 0;
    ctrl->cfg_focus = 0;

    if (now_ok && now_tm) {
        ctrl->cfg_hh = now_tm->tm_hour;
        ctrl->cfg_mm = now_tm->tm_min;
    } else {
        ctrl->cfg_hh = 12;
        ctrl->cfg_mm = 0;
    }
}

bool ui_controller_on_encoder(ui_controller_t *ctrl, int32_t steps) {
    if (!ctrl || steps == 0) return false;

    int direction = (steps > 0) ? 1 : -1;

    if (ctrl->screen == SCREEN_MENU_ROOT) {
        ctrl->sel_root = (ctrl->sel_root + direction) & 1;
    } else if (ctrl->screen == SCREEN_MENU_MODE) {
        ctrl->sel_mode = (ctrl->sel_mode + direction) & 1;
    } else if (ctrl->screen == SCREEN_MENU_STAGE) {
        ctrl->sel_stage = (ctrl->sel_stage + direction) & 1;
    } else if (ctrl->screen == SCREEN_CONFIG_TIME) {
        if (ctrl->cfg_focus == 0) {
            ctrl->cfg_hh += direction;
            if (ctrl->cfg_hh < 0) ctrl->cfg_hh = 23;
            if (ctrl->cfg_hh > 23) ctrl->cfg_hh = 0;
        } else {
            ctrl->cfg_mm += direction;
            if (ctrl->cfg_mm < 0) ctrl->cfg_mm = 59;
            if (ctrl->cfg_mm > 59) ctrl->cfg_mm = 0;
        }
    }

    return true;
}

static void apply_time_config(const ui_controller_t *ctrl) {
    struct tm new_tm = {0};
    new_tm.tm_year = 2025 - 1900;
    new_tm.tm_mon = 0;
    new_tm.tm_mday = 1;
    new_tm.tm_hour = ctrl->cfg_hh;
    new_tm.tm_min = ctrl->cfg_mm;
    new_tm.tm_sec = 0;
    rtc_ds3231_write_tm(&new_tm);
}

bool ui_controller_on_click(ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok) {
    if (!ctrl) return false;

    if (ctrl->screen == SCREEN_STATUS) {
        ctrl->sel_root = 0;
        ctrl->screen = SCREEN_MENU_ROOT;
        return true;
    }

    if (ctrl->screen == SCREEN_MENU_ROOT) {
        if (ctrl->sel_root == 0) {
            ctrl->sel_mode = (state && state->mode == GROW_MODE_PHOTO) ? 1 : 0;
            ctrl->sel_stage = (state && state->stage == STAGE_FLOR) ? 1 : 0;
            ctrl->screen = SCREEN_MENU_MODE;
        } else {
            if (now_ok && now_tm) {
                ctrl->cfg_hh = now_tm->tm_hour;
                ctrl->cfg_mm = now_tm->tm_min;
            } else {
                ctrl->cfg_hh = 12;
                ctrl->cfg_mm = 0;
            }
            ctrl->cfg_focus = 0;
            ctrl->screen = SCREEN_CONFIG_TIME;
        }
        return true;
    }

    if (ctrl->screen == SCREEN_CONFIG_TIME) {
        if (ctrl->cfg_focus == 0) {
            ctrl->cfg_focus = 1;
        } else {
            apply_time_config(ctrl);
            ctrl->screen = SCREEN_STATUS;
        }
        return true;
    }

    if (ctrl->screen == SCREEN_MENU_MODE) {
        grow_mode_t new_mode = (ctrl->sel_mode == 0) ? GROW_MODE_AUTO : GROW_MODE_PHOTO;
        app_state_set_mode(new_mode, now_tm, now_ok);
        ctrl->screen = (new_mode == GROW_MODE_PHOTO) ? SCREEN_MENU_STAGE : SCREEN_STATUS;
        return true;
    }

    // SCREEN_MENU_STAGE
    stage_t new_stage = (ctrl->sel_stage == 0) ? STAGE_VEG : STAGE_FLOR;
    app_state_set_stage(new_stage, now_tm, now_ok);
    ctrl->screen = SCREEN_STATUS;
    return true;
}

void ui_controller_render(const ui_controller_t *ctrl, const state_t *state, const struct tm *now_tm, bool now_ok) {
    if (!ctrl || !state) return;

    switch (ctrl->screen) {
        case SCREEN_STATUS:
            screen_render_status_simple(state, now_ok ? now_tm : NULL, now_ok);
            break;
        case SCREEN_MENU_ROOT:
            screen_render_menu_root(ctrl->sel_root);
            break;
        case SCREEN_CONFIG_TIME:
            screen_render_config_time(ctrl->cfg_hh, ctrl->cfg_mm, ctrl->cfg_focus);
            break;
        case SCREEN_MENU_MODE:
            screen_render_menu_mode(ctrl->sel_mode);
            break;
        case SCREEN_MENU_STAGE:
        default:
            screen_render_menu_stage(ctrl->sel_stage);
            break;
    }
}
