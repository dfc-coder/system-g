// main/app_state.h
#pragma once

#include <stdint.h>
#include <stdbool.h>
#include <time.h>

typedef enum { GROW_MODE_AUTO = 0, GROW_MODE_PHOTO = 1 } grow_mode_t;
typedef enum { STAGE_VEG = 0, STAGE_FLOR = 1 } stage_t;

typedef struct {
    grow_mode_t mode;
    stage_t     stage;

    // schedule diario por reloj
    uint16_t start_min_of_day;  // 0..1439
    uint16_t on_min_per_day;    // 0..1440

    bool relay_on;
    bool time_valid;

    // ciclo (edad del ciclo)
    bool    cycle_start_set;
    int32_t cycle_start_daycount; // día absoluto

    // debug simple
    int32_t enc_total_steps;
    char    last_evt; // '<' '>' 'B' '-'

    // DHT22
    bool  dht_ok;
    float dht_temp_c;
    float dht_hum_pct;
    int64_t dht_last_read_ms;
} state_t;

/* init & snapshots */
void app_state_init_defaults(void);
void app_state_get_snapshot(state_t *out);

/* setters (internamente thread-safe) */
void app_state_set_time_valid(bool ok);
void app_state_set_relay_on(bool on);
void app_state_add_encoder_steps(int32_t steps);
void app_state_set_last_evt(char evt);

void app_state_set_mode(grow_mode_t mode, const struct tm *now_tm, bool now_ok);
void app_state_set_stage(stage_t stage, const struct tm *now_tm, bool now_ok);

/* schedule helpers */
bool app_state_schedule_should_on(int now_min, uint16_t start_min, uint16_t on_min);
uint16_t app_state_hhmm_to_min(uint8_t hh, uint8_t mm);

/* DHT22 */
void app_state_set_dht_reading(bool ok, float temp_c, float hum_pct, int64_t ts_ms);
