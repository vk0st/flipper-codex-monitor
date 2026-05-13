#pragma once

#include <furi.h>
#include <furi_hal.h>
#include <furi_hal_bt.h>
#include "helpers/ble_serial.h"
#include <bt/bt_service/bt.h>
#include <gui/gui.h>
#include <gui/elements.h>
#include <notification/notification_messages.h>
#include <input/input.h>
#include <storage/storage.h>

#include "views/bars_view.h"
#include "views/connect_view.h"
#include "views/status_view.h"

#define TAG                   "CodexMonitor"
#define BT_SERIAL_BUFFER_SIZE 128

#define SCREEN_HEIGHT 64
#define LINE_HEIGHT   11

#define BAR_X     20
#define BAR_WIDTH 107

#define CODEX_STATUS_OK            0
#define CODEX_STATUS_STALE         1
#define CODEX_STATUS_CODEX_ERROR   2
#define CODEX_STATUS_LIMIT_REACHED 3

typedef enum {
    BtStateChecking,
    BtStateInactive,
    BtStateWaiting,
    BtStateReceiving,
    BtStateNoData,
    BtStateLost
} BtState;

#pragma pack(push, 1)
typedef struct {
    uint8_t five_hour_used_percent;
    char five_hour_reset[8];
    uint8_t week_used_percent;
    char week_reset[10];
    uint8_t status;
} CodexLimitsPacket;
#pragma pack(pop)

typedef struct {
    uint8_t type;
    CodexLimitsPacket packet;
} CodexMonitorEvent;

typedef struct {
    Bt* bt;
    Gui* gui;
    ViewPort* view_port;
    FuriMessageQueue* event_queue;
    FuriMessageQueue* bt_event_queue;
    NotificationApp* notification;
    FuriHalBleProfileBase* ble_serial_profile;

    BtState bt_state;
    CodexLimitsPacket data;
    uint32_t last_packet;
} CodexMonitorApp;
