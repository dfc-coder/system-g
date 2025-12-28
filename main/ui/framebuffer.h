#pragma once

#include <stdint.h>

void fb_clear(void);
void fb_draw_text(uint8_t x, uint8_t page, const char *s);

const uint8_t *fb_pages_flattened(void); // pages contiguos
int fb_pages_count(void);
int fb_width(void);