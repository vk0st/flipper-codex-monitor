use btleplug::api::{Central, Manager as _, Peripheral as _};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use uuid::Uuid;

pub const FLIPPER_CHARACTERISTIC_UUID: Uuid =
    Uuid::from_u128(0x19ed82ae_ed21_4c9d_4145_228e62fe0000);

pub async fn get_central(manager: &Manager) -> Adapter {
    manager
        .adapters()
        .await
        .unwrap()
        .into_iter()
        .nth(0)
        .unwrap()
}

pub async fn get_flipper(central: &Adapter, id: &PeripheralId) -> Option<Peripheral> {
    for p in central
        .peripherals()
        .await
        .unwrap()
        .iter()
        .filter(|p| p.id() == *id)
    {
        if has_codex_monitor_name(p).await {
            return Some(p.clone());
        }
    }
    None
}

pub async fn connect_known_flippers(central: &Adapter) {
    let Ok(peripherals) = central.peripherals().await else {
        return;
    };

    for peripheral in peripherals {
        if !has_codex_monitor_name(&peripheral).await {
            continue;
        }

        match peripheral.is_connected().await {
            Ok(true) => {}
            Ok(false) => {
                println!("[{}] Connecting to Flipper", peripheral.id().to_string());
                if let Err(e) = peripheral.connect().await {
                    println!(
                        "[{}] Failed to connect to Flipper: {}",
                        peripheral.id().to_string(),
                        e
                    );
                }
            }
            Err(e) => {
                println!(
                    "[{}] Failed to read connection state: {}",
                    peripheral.id().to_string(),
                    e
                );
            }
        }
    }
}

async fn has_codex_monitor_name(peripheral: &Peripheral) -> bool {
    peripheral
        .properties()
        .await
        .ok()
        .flatten()
        .as_ref()
        .is_some_and(has_codex_monitor_properties)
}

fn has_codex_monitor_properties(properties: &btleplug::api::PeripheralProperties) -> bool {
    properties
        .local_name
        .as_deref()
        .is_some_and(|name| name.contains("Codex"))
}
