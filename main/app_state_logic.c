#include "app_state_logic.h"

#include <string.h>

#include "app_config.h"

uint16_t app_state_hhmm_to_min(uint8_t hh, uint8_t mm) {
    return (uint16_t)((hh % 24) * 60 + (mm % 60));
}

bool app_state_schedule_should_on(int now_min, uint16_t start_min, uint16_t on_min) {
    if (on_min == 0) return false;
    if (on_min >= 1440) return true;

    int end = (start_min + on_min) % 1440;

    if ((int)start_min < end) {
        return (now_min >= (int)start_min) && (now_min < end);
    }
    return (now_min >= (int)start_min) || (now_min < end);
}

void app_state_apply_profile_schedule(state_t *st) {
    if (!st) return;

    if (st->mode == GROW_MODE_AUTO) {
        st->start_min_of_day = app_state_hhmm_to_min(START_AUTO_HH, START_AUTO_MM);
        st->on_min_per_day   = 20 * 60;
        return;
    }

    if (st->stage == STAGE_FLOR) {
        st->start_min_of_day = app_state_hhmm_to_min(START_FLOR_HH, START_FLOR_MM);
        st->on_min_per_day   = 12 * 60;
        return;
    }

    st->start_min_of_day = app_state_hhmm_to_min(START_VEG_HH, START_VEG_MM);
    st->on_min_per_day   = 18 * 60;
}

void app_state_reset_defaults(state_t *st) {
    if (!st) return;

    memset(st, 0, sizeof(*st));

    st->mode = GROW_MODE_AUTO;
    st->stage = STAGE_VEG;

    st->relay_on = false;
    st->time_valid = false;

    st->cycle_start_set = false;
    st->cycle_start_daycount = 0;

    st->enc_total_steps = 0;
    st->last_evt = '-';

    st->dht_ok = false;
    st->dht_temp_c = 0.0f;
    st->dht_hum_pct = 0.0f;
    st->dht_last_read_ms = 0;

    app_state_apply_profile_schedule(st);
}
