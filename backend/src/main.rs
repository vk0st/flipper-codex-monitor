use btleplug::api::{Central, CentralEvent, Characteristic, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use flipper_codex_monitor_backend::codex_app_server::{self, CodexLimitsCache};
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use tokio::time::{self, Duration};

mod flipper_manager;

async fn data_sender(
    flipper: Peripheral,
    cmd_char: Characteristic,
    limits_cache: CodexLimitsCache,
) {
    let id = flipper.id();
    println!("[{}] Sending data...", id.to_string());

    loop {
        let packet = limits_cache.current_packet().await;
        let packet_bytes = bincode::serialize(&packet).unwrap();

        if let Err(e) = flipper
            .write(
                &cmd_char,
                &packet_bytes,
                btleplug::api::WriteType::WithoutResponse,
            )
            .await
        {
            println!("[{}] Failed to write: {}", id.to_string(), e);
        };

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

async fn find_serial_characteristic(flipper: &Peripheral) -> Option<Characteristic> {
    let id = flipper.id();

    for attempt in 1..=8 {
        if let Err(e) = flipper.discover_services().await {
            println!(
                "[{}] Service discovery failed ({}/8): {}",
                id.to_string(),
                attempt,
                e
            );
        }

        if let Some(characteristic) = flipper
            .characteristics()
            .into_iter()
            .find(|c| c.uuid == flipper_manager::FLIPPER_CHARACTERISTIC_UUID)
        {
            return Some(characteristic);
        }

        println!(
            "[{}] Waiting for Flipper serial characteristic ({}/8)",
            id.to_string(),
            attempt
        );
        tokio::time::sleep(Duration::from_millis(750)).await;
    }

    None
}

async fn reconnect_thread(central: Adapter, id: PeripheralId) {
    loop {
        if let Some(flipper) = flipper_manager::get_flipper(&central, &id).await {
            let _ = flipper.connect().await;
        };

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

async fn connect_discovered_flipper(central: &Adapter, id: &PeripheralId) {
    if let Some(flp) = flipper_manager::get_flipper(central, id).await {
        if matches!(flp.is_connected().await, Ok(true)) {
            return;
        }

        println!("[{}] Connecting to Flipper", id.to_string());
        if let Err(e) = flp.connect().await {
            println!("[{}] Failed to connect to Flipper: {}", id.to_string(), e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    pretty_env_logger::init();

    if std::env::args().any(|arg| arg == "--smoke-test") {
        return codex_app_server::run_smoke_test().await;
    }

    let manager = Manager::new().await?;

    let central = flipper_manager::get_central(&manager).await;
    println!("Found {:?} adapter", central.adapter_info().await.unwrap());

    let mut events = central.events().await?;
    let limits_cache = CodexLimitsCache::start_polling();

    println!("Scanning... Launch Codex Monitor app on Flipper");
    central.start_scan(ScanFilter::default()).await?;

    let mut data_workers: HashMap<PeripheralId, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut reconnect_workers: HashMap<PeripheralId, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut discovery_interval = time::interval(Duration::from_secs(5));

    flipper_manager::connect_known_flippers(&central).await;

    loop {
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else {
                    break;
                };

                match event {
                    CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                        connect_discovered_flipper(&central, &id).await;
                    }
                    CentralEvent::DeviceConnected(id) => {
                        if let Some(flp) = flipper_manager::get_flipper(&central, &id).await {
                            println!("[{}] Connected to Flipper", &id.to_string());

                            if !data_workers.contains_key(&id) {
                                if let Some(cmd_char) = find_serial_characteristic(&flp).await {
                                    data_workers.insert(
                                        id.clone(),
                                        tokio::spawn(data_sender(
                                            flp,
                                            cmd_char,
                                            limits_cache.clone(),
                                        )),
                                    );
                                } else {
                                    println!(
                                        "[{}] Failed to find Flipper serial characteristic",
                                        id.to_string()
                                    );
                                    let _ = flp.disconnect().await;
                                }
                            }
                        };

                        match reconnect_workers.get(&id) {
                            Some(worker) => {
                                worker.abort();
                                reconnect_workers.remove(&id);
                            }
                            None => {}
                        }
                    }
                    CentralEvent::DeviceDisconnected(id) => {
                        match data_workers.get(&id) {
                            Some(worker) => {
                                worker.abort();
                                println!(
                                    "[{}] Disconnected from Flipper. Waiting for reconnection",
                                    &id.to_string()
                                );

                                data_workers.remove(&id);
                            }
                            None => {}
                        };

                        reconnect_workers.insert(
                            id.clone(),
                            tokio::spawn(reconnect_thread(central.clone(), id)),
                        );
                    }
                    _ => {}
                }
            }
            _ = discovery_interval.tick() => {
                flipper_manager::connect_known_flippers(&central).await;
            }
        };
    }

    Ok(())
}
