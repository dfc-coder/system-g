#include "drivers/relay.h"

#include "driver/gpio.h"
#include "app_config.h"

static bool g_relay_on = false;

static inline int relay_level_from_on(bool on) {
    int level = on ? 1 : 0;
    if (RELAY_ACTIVE_LOW) level = !level;
    return level;
}

void relay_init(void) {
    gpio_config_t io = {
        .pin_bit_mask = 1ULL << RELAY_GPIO,
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&io);

    // OFF inmediato (evita pulso al boot)
    g_relay_on = false;
    gpio_set_level(RELAY_GPIO, relay_level_from_on(false));
}

void relay_set(bool on) {
    g_relay_on = on;
    gpio_set_level(RELAY_GPIO, relay_level_from_on(on));
}

bool relay_get(void) {
    return g_relay_on;
}
