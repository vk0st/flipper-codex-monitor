#include "bars_view.h"

static void copy_packet_string(char* dst, size_t dst_size, const char* src, size_t src_size) {
    size_t i = 0;
    while(i + 1 < dst_size && i < src_size && src[i] != '\0') {
        dst[i] = src[i];
        i++;
    }
    dst[i] = '\0';
}

static void draw_limit_row(
    Canvas* canvas,
    uint8_t y,
    const char* label,
    uint8_t used_percent,
    const char* reset,
    size_t reset_size) {
    char reset_str[11];
    char bar_text[20];

    copy_packet_string(reset_str, sizeof(reset_str), reset, reset_size);
    snprintf(bar_text, sizeof(bar_text), "%u%% %s", used_percent, reset_str);

    canvas_draw_str(canvas, 1, y + 9, label);
    elements_progress_bar_with_text(
        canvas, BAR_X, y, BAR_WIDTH, used_percent / 100.0f, bar_text);
}

void draw_bars_view(Canvas* canvas, void* ctx) {
    CodexMonitorApp* app = ctx;

    canvas_clear(canvas);
    canvas_set_color(canvas, ColorBlack);
    canvas_set_font(canvas, FontKeyboard);

    if(app->data.status == CODEX_STATUS_CODEX_ERROR) {
        canvas_draw_str_aligned(canvas, 64, 32, AlignCenter, AlignCenter, "Codex error");
        return;
    }

    draw_limit_row(
        canvas,
        12,
        "5H",
        app->data.five_hour_used_percent,
        app->data.five_hour_reset,
        sizeof(app->data.five_hour_reset));
    draw_limit_row(
        canvas,
        34,
        "1W",
        app->data.week_used_percent,
        app->data.week_reset,
        sizeof(app->data.week_reset));

    canvas_set_font(canvas, FontSecondary);
    if(app->data.status == CODEX_STATUS_STALE) {
        canvas_draw_str_aligned(canvas, 64, 63, AlignCenter, AlignBottom, "stale data");
    } else if(app->data.status == CODEX_STATUS_LIMIT_REACHED) {
        canvas_draw_str_aligned(canvas, 64, 63, AlignCenter, AlignBottom, "limit reached");
    }
}
