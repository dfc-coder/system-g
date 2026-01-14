#pragma once

#include "app_state.h"

// Helpers to keep app_state.c small and focused.
void app_state_reset_defaults(state_t *st);
void app_state_apply_profile_schedule(state_t *st);
