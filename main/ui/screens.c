#include "ui/screens.h"

#include <stdio.h>
#include <string.h>

#include "ui/framebuffer.h"
#include "drivers/rtc_ds3231.h"

static const char *stage_str(stage_t s) {
    return (s == STAGE_VEG) ? "VEG" : "FLOR";
}

static void hhmm_from_min_of_day(uint16_t mod, char out[6]) {
    unsigned hh = (mod / 60) % 24;
    unsigned mm = mod % 60;
    snprintf(out, 6, "%02u:%02u", hh, mm);
}

static void profile_str(const state_t *st, char out[16]) {
    if (st->mode == GROW_MODE_AUTO) {
        snprintf(out, 16, "AUTO");
        return;
    }
    snprintf(out, 16, "PHOTO-%s", stage_str(st->stage));
}

static void format_age(const state_t *st, const struct tm *now_tm, bool now_ok, char out[16]) {
    if (!st->cycle_start_set || !now_ok || !now_tm) {
        snprintf(out, 16, "--");
        return;
    }

    int32_t now_dc = rtc_daycount_from_tm(now_tm);
    int32_t diff = now_dc - st->cycle_start_daycount;
    if (diff < 0) diff = 0;

    int32_t w = diff / 7;
    int32_t d = diff % 7;

    if (w > 0) snprintf(out, 16, "%ldw %ldd", (long)w, (long)d);
    else       snprintf(out, 16, "%ldd", (long)d);
}

void screen_render_status_simple(const state_t *st, const struct tm *now_tm, bool now_ok) {
    char line[32];

    char prof[16];
    char start_hhmm[6];
    char age[16];

    profile_str(st, prof);
    hhmm_from_min_of_day(st->start_min_of_day, start_hhmm);
    format_age(st, now_tm, now_ok, age);

    fb_clear();
    fb_draw_text(0, 0, "GROW");

    if (now_ok && now_tm) snprintf(line, sizeof(line), "TIME %02d:%02d", now_tm->tm_hour, now_tm->tm_min);
    else                                   snprintf(line, sizeof(line), "TIME --:--");
    fb_draw_text(0, 1, line);

    snprintf(line, sizeof(line), "CYCLE %s", prof);
    fb_draw_text(0, 3, line);

    snprintf(line, sizeof(line), "START %s", start_hhmm);
    fb_draw_text(0, 5, line);

    snprintf(line, sizeof(line), "AGE %s", age);
    fb_draw_text(0, 6, line);

    snprintf(line, sizeof(line), "LIGHT %s", st->relay_on ? "ON" : "OFF");
    fb_draw_text(0, 7, line);
}

void screen_render_menu_mode(int sel) {
    fb_clear();
    fb_draw_text(0, 0, "SELECT MODE");
    fb_draw_text(0, 2, (sel == 0) ? "> AUTO"  : "  AUTO");
    fb_draw_text(0, 3, (sel == 1) ? "> PHOTO" : "  PHOTO");
    fb_draw_text(0, 7, "BTN=OK  ROT=SEL");
}

void screen_render_menu_stage(int sel) {
    fb_clear();
    fb_draw_text(0, 0, "PHOTO STAGE");
    fb_draw_text(0, 2, (sel == 0) ? "> VEG  18/6" : "  VEG  18/6");
    fb_draw_text(0, 3, (sel == 1) ? "> FLOR 12/12": "  FLOR 12/12");
    fb_draw_text(0, 7, "BTN=OK  ROT=SEL");
}

void screen_render_menu_root(int sel) {
    fb_clear();
    fb_draw_text(0, 0, "MAIN MENU");
    fb_draw_text(0, 2, (sel == 0) ? "> GROW CONFIG" : "  GROW CONFIG");
    fb_draw_text(0, 3, (sel == 1) ? "> SET TIME"    : "  SET TIME");
    fb_draw_text(0, 7, "BTN=OK  ROT=SEL");
}

void screen_render_config_time(int hh, int mm, int focus_idx) {
    fb_clear();
    fb_draw_text(0, 0, "SET TIME");

    char line[16];
    // Simple highlighting logic
    if (focus_idx == 0) {
        snprintf(line, sizeof(line), ">%02d< : %02d", hh, mm);
    } else {
        snprintf(line, sizeof(line), " %02d : >%02d<", hh, mm);
    }
    fb_draw_text(2, 3, line);

    fb_draw_text(0, 7, (focus_idx == 0) ? "SET HOUR" : "SET MINUTE");
}