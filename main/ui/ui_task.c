#include "ui/ui_task.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "esp_timer.h"
#include "esp_log.h"
#include "esp_err.h"

#include "app_config.h"
#include "app_state.h"

#include "drivers/oled_sh1106.h"
#include "drivers/i2c_bus.h"
#include "drivers/rtc_ds3231.h"

#include "tasks/input_task.h"

#include "ui/framebuffer.h"
#include "ui/screens.h"

static const char *TAG = APP_TAG;

static TaskHandle_t g_ui_handle = NULL;

void ui_task_notify_redraw(void) {
    if (g_ui_handle) {
        xTaskNotifyGive(g_ui_handle);
    }
}

static void ui_flush_or_reset_bus(void) {
    esp_err_t e = oled_sh1106_flush_pages(
        fb_pages_flattened(),
        fb_pages_count(),
        fb_width()
    );

    if (e != ESP_OK) {
        ESP_LOGW(TAG, "OLED flush failed: %s (bus reset)", esp_err_to_name(e));
        i2c_bus_reset();
    }
}

static void ui_task(void *arg) {
    (void)arg;

    g_ui_handle = xTaskGetCurrentTaskHandle();

    screen_t screen = SCREEN_STATUS;
    int sel_mode = 0;
    int sel_stage = 0;
    int sel_root = 0;
    
    // Time config state
    int cfg_hh = 12;
    int cfg_mm = 0;
    int cfg_focus = 0; // 0=HH, 1=MM

    int64_t last_draw_us = 0;

    // --- first draw ---
    state_t st;
    struct tm now_tm = {0};
    bool ok = rtc_time_now_local(&now_tm);

    app_state_get_snapshot(&st);
    screen_render_status_simple(&st, ok ? &now_tm : NULL, ok);
    ui_flush_or_reset_bus();

    while (1) {
        // notified > 0 => alguien pidió redraw explícito (DHT/control/input)
        uint32_t notified = ulTaskNotifyTake(pdTRUE, pdMS_TO_TICKS(500));

        // throttle de redraw
        int64_t now_us = esp_timer_get_time();
        if ((now_us - last_draw_us) < (UI_MIN_REDRAW_MS * 1000LL)) {
            TickType_t tt = pdMS_TO_TICKS(UI_MIN_REDRAW_MS);
            if (tt == 0) tt = 1;
            vTaskDelay(tt);
        }
        last_draw_us = esp_timer_get_time();

        // leer eventos de input (no pises dirty del notify)
        int32_t delta_steps = 0;
        bool click = false;
        bool dirty_input = false;

        input_task_pop_events(&delta_steps, &click, &dirty_input);

        bool dirty = (notified > 0) || dirty_input;

        // si no hay nada que procesar, no redibujes
        if (!dirty && delta_steps == 0 && !click) {
            continue;
        }

        // aplicar cambios de input al estado
        if (delta_steps != 0) {
            app_state_add_encoder_steps(delta_steps);
            app_state_set_last_evt((delta_steps > 0) ? '>' : '<');
            dirty = true;
        }

        if (click) {
            app_state_set_last_evt('B');
            dirty = true;
        }

        // navegación de menús (solo con encoder)
        if (screen == SCREEN_MENU_ROOT && delta_steps != 0) {
            sel_root = (sel_root + (delta_steps > 0 ? 1 : -1)) & 1;
            dirty = true;
        } else if (screen == SCREEN_MENU_MODE && delta_steps != 0) {
            sel_mode = (sel_mode + (delta_steps > 0 ? 1 : -1)) & 1;
            dirty = true;
        } else if (screen == SCREEN_MENU_STAGE && delta_steps != 0) {
            sel_stage = (sel_stage + (delta_steps > 0 ? 1 : -1)) & 1;
            dirty = true;
        } else if (screen == SCREEN_CONFIG_TIME && delta_steps != 0) {
            if (cfg_focus == 0) { // HH
                cfg_hh += (delta_steps > 0 ? 1 : -1);
                if (cfg_hh < 0) cfg_hh = 23;
                if (cfg_hh > 23) cfg_hh = 0;
            } else { // MM
                cfg_mm += (delta_steps > 0 ? 1 : -1);
                if (cfg_mm < 0) cfg_mm = 59;
                if (cfg_mm > 59) cfg_mm = 0;
            }
            dirty = true;
        }

        // leer hora (si falla, ok=false y se renderiza --:--)
        ok = rtc_time_now_local(&now_tm);

        // click: transiciones y persistencia en app_state
        if (click) {
            if (screen == SCREEN_STATUS) {
                sel_root = 0;
                screen = SCREEN_MENU_ROOT;
            
            } else if (screen == SCREEN_MENU_ROOT) {
                if (sel_root == 0) {
                    // GROW CONFIG
                    app_state_get_snapshot(&st);
                    sel_mode  = (st.mode  == GROW_MODE_AUTO) ? 0 : 1;
                    sel_stage = (st.stage == STAGE_VEG) ? 0 : 1;
                    screen = SCREEN_MENU_MODE;
                } else {
                    // SET TIME
                    if (ok) {
                        cfg_hh = now_tm.tm_hour;
                        cfg_mm = now_tm.tm_min;
                    } else {
                        cfg_hh = 12;
                        cfg_mm = 0;
                    }
                    cfg_focus = 0;
                    screen = SCREEN_CONFIG_TIME;
                }

            } else if (screen == SCREEN_CONFIG_TIME) {
                if (cfg_focus == 0) {
                    cfg_focus = 1; // set minute
                } else {
                    // SAVE
                    struct tm new_tm = {0};
                    new_tm.tm_year = 2025 - 1900; // default year
                    new_tm.tm_mon = 0;
                    new_tm.tm_mday = 1;
                    new_tm.tm_hour = cfg_hh;
                    new_tm.tm_min = cfg_mm;
                    new_tm.tm_sec = 0;
                    rtc_ds3231_write_tm(&new_tm);
                    
                    // update system time immediately?
                    // rtc_time_now_local will read back from RTC anyway
                    
                    screen = SCREEN_STATUS;
                }

            } else if (screen == SCREEN_MENU_MODE) {
                grow_mode_t nm = (sel_mode == 0) ? GROW_MODE_AUTO : GROW_MODE_PHOTO;
                app_state_set_mode(nm, ok ? &now_tm : NULL, ok);
                screen = (nm == GROW_MODE_PHOTO) ? SCREEN_MENU_STAGE : SCREEN_STATUS;
            } else { // SCREEN_MENU_STAGE
                stage_t ns = (sel_stage == 0) ? STAGE_VEG : STAGE_FLOR;
                app_state_set_stage(ns, ok ? &now_tm : NULL, ok);
                screen = SCREEN_STATUS;
            }
            dirty = true;
        }

        // si después de procesar todo no quedó dirty, salí
        if (!dirty) {
            continue;
        }

        // render
        app_state_get_snapshot(&st);

        if (screen == SCREEN_STATUS) {
            screen_render_status_simple(&st, ok ? &now_tm : NULL, ok);
        } else if (screen == SCREEN_MENU_ROOT) {
            screen_render_menu_root(sel_root);
        } else if (screen == SCREEN_CONFIG_TIME) {
            screen_render_config_time(cfg_hh, cfg_mm, cfg_focus);
        } else if (screen == SCREEN_MENU_MODE) {
            screen_render_menu_mode(sel_mode);
        } else {
            screen_render_menu_stage(sel_stage);
        }

        ui_flush_or_reset_bus();
    }
}

void ui_task_start(void) {
    xTaskCreate(ui_task, "ui", 4096, NULL, PRIO_UI, NULL);
}
