#include "codex_monitor.h"

static void render_callback(Canvas* canvas, void* ctx) {
    furi_assert(ctx);
    CodexMonitorApp* app = ctx;

    switch(app->bt_state) {
    case BtStateWaiting:
        draw_connect_view(canvas);
        break;

    case BtStateReceiving:
        draw_bars_view(canvas, app);
        break;

    default:
        draw_status_view(canvas, app);
        break;
    }
}

static void input_callback(InputEvent* input_event, void* ctx) {
    furi_assert(ctx);
    FuriMessageQueue* event_queue = ctx;
    furi_message_queue_put(event_queue, input_event, FuriWaitForever);
}

static uint16_t bt_serial_callback(SerialServiceEvent event, void* ctx) {
    furi_assert(ctx);
    CodexMonitorApp* app = ctx;

    if(event.event == SerialServiceEventTypeDataReceived) {
        FURI_LOG_D(
            TAG,
            "SerialServiceEventTypeDataReceived. Size: %u/%u",
            event.data.size,
            sizeof(CodexLimitsPacket));

        if(event.data.size == sizeof(CodexLimitsPacket)) {
            CodexMonitorEvent app_event = {
                .type = SerialServiceEventTypeDataReceived,
            };
            memcpy(&app_event.packet, event.data.buffer, sizeof(CodexLimitsPacket));
            furi_message_queue_put(app->bt_event_queue, &app_event, 0);
        }
    }

    return BT_SERIAL_BUFFER_SIZE;
}

static CodexMonitorApp* codex_monitor_alloc() {
    CodexMonitorApp* app = malloc(sizeof(CodexMonitorApp));
    app->view_port = view_port_alloc();
    app->event_queue = furi_message_queue_alloc(8, sizeof(InputEvent));
    app->bt_event_queue = furi_message_queue_alloc(8, sizeof(CodexMonitorEvent));
    app->notification = furi_record_open(RECORD_NOTIFICATION);
    app->gui = furi_record_open(RECORD_GUI);
    app->bt = furi_record_open(RECORD_BT);

    gui_add_view_port(app->gui, app->view_port, GuiLayerFullscreen);
    view_port_draw_callback_set(app->view_port, render_callback, app);
    view_port_input_callback_set(app->view_port, input_callback, app->event_queue);
    return app;
}

static void codex_monitor_free(CodexMonitorApp* app) {
    gui_remove_view_port(app->gui, app->view_port);
    view_port_free(app->view_port);
    furi_message_queue_free(app->event_queue);
    furi_message_queue_free(app->bt_event_queue);
    furi_record_close(RECORD_NOTIFICATION);
    furi_record_close(RECORD_GUI);
    furi_record_close(RECORD_BT);
    free(app);
}

int32_t codex_monitor_app(void* p) {
    UNUSED(p);
    CodexMonitorApp* app = codex_monitor_alloc();

    bt_disconnect(app->bt);

    // Wait 2nd core to update nvm storage
    furi_delay_ms(200);

    bt_keys_storage_set_storage_path(app->bt, APP_DATA_PATH(".codex_monitor.keys"));

    BleProfileSerialParams params = {
        .device_name_prefix = "Codex",
        .mac_xor = 0x0003,
    };
    app->ble_serial_profile = bt_profile_start(app->bt, ble_profile_serial, &params);

    furi_check(app->ble_serial_profile);

    ble_profile_serial_set_event_callback(
        app->ble_serial_profile, BT_SERIAL_BUFFER_SIZE, bt_serial_callback, app);
    furi_hal_bt_start_advertising();

    app->bt_state = BtStateWaiting;
    FURI_LOG_D(TAG, "Bluetooth is active!");

    // Main loop
    InputEvent event;
    CodexMonitorEvent app_event;
    while(true) {
        if(furi_message_queue_get(app->event_queue, &event, 1) == FuriStatusOk) {
            if(event.type == InputTypeShort && event.key == InputKeyBack) break;
        }

        if(furi_message_queue_get(app->bt_event_queue, &app_event, 0) == FuriStatusOk) {
            if(app_event.type == SerialServiceEventTypeDataReceived) {
                memcpy(&app->data, &app_event.packet, sizeof(CodexLimitsPacket));
                app->bt_state = BtStateReceiving;
                app->last_packet = furi_hal_rtc_get_timestamp();
                view_port_update(app->view_port);
                ble_profile_serial_notify_buffer_is_empty(app->ble_serial_profile);

                // Elegant solution, the backlight is only on when there is continuous communication
                notification_message(app->notification, &sequence_display_backlight_on);

                notification_message(app->notification, &sequence_blink_blue_10);
            }
        }

        if(app->bt_state == BtStateReceiving &&
           (furi_hal_rtc_get_timestamp() - app->last_packet > 5)) {
            app->bt_state = BtStateLost;
            view_port_update(app->view_port);
        }
    }

    ble_profile_serial_set_event_callback(app->ble_serial_profile, 0, NULL, NULL);

    bt_disconnect(app->bt);

    // Wait 2nd core to update nvm storage
    furi_delay_ms(200);

    bt_keys_storage_set_default_path(app->bt);

    furi_check(bt_profile_restore_default(app->bt));

    codex_monitor_free(app);

    return 0;
}
