idf_component_register(
    SRCS
        "main.c"
        "app_state.c"
        "app_state_logic.c"
        "app_state_storage.c"

        "drivers/i2c_bus.c"
        "drivers/oled_sh1106.c"
        "drivers/relay.c"
        "drivers/rtc_ds3231.c"
        "drivers/encoder_pcnt.c"
        "drivers/dht22.c"

        "tasks/input_task.c"
        "tasks/control_task.c"
        "tasks/dht_task.c"

        "ui/framebuffer.c"
        "ui/screens.c"
        "ui/ui_task.c"
    
    INCLUDE_DIRS "."
    PRIV_REQUIRES
        nvs_flash
        log
        button
        esp_driver_gpio
        esp_timer
        esp_driver_i2c
        esp_driver_pcnt
)
