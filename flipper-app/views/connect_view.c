#include "connect_view.h"

void draw_connect_view(Canvas* canvas) {
    canvas_clear(canvas);
    canvas_set_color(canvas, ColorBlack);
    canvas_set_font(canvas, FontPrimary);

    canvas_draw_str(canvas, 1, 12, "Waiting for backend...");

    canvas_set_font(canvas, FontSecondary);
    canvas_draw_str(canvas, 1, 36, "Run:");
    canvas_draw_str(canvas, 1, 48, "flipper-codex-monitor");
    canvas_draw_str(canvas, 1, 60, "backend on your PC");
}
