use bluez_async::{
    BleUuid, BluetoothEvent, BluetoothSession, CharacteristicEvent, CharacteristicFlags,
    CharacteristicInfo,
};
use futures::stream::StreamExt;
use std::process::exit;
use std::str;
use std::time::Duration;
use tokio::time;

use std::fs;
use toml;

use serde::{Deserialize};

#[derive(Deserialize)]
struct Config {
    zabbix: ZabbixConfig,
    device: DeviceConfig,
    bluetooth: BluetoothConfig
}
#[derive(Deserialize)]
struct ZabbixConfig {
    address: String,
    port: u16,
    host: String,
    item: String,
    send_timeout: u32,
}
#[derive(Deserialize)]
struct DeviceConfig {
    mac_address: String,
}
#[derive(Deserialize)]
struct BluetoothConfig {
    scan_duration: u16
}

// pub mod ble;
mod packet;
use packet::CellsInfo;

use serde_json::to_string;
use zbx_sender::Sender;

fn load_config(filename: &str) -> Config {
    let contents = match fs::read_to_string(filename) {
        // If successful return the files text as `contents`.
        // `c` is a local variable.
        Ok(c) => c,
        // Handle the `error` case.
        Err(_) => {
            // Write `msg` to `stderr`.
            eprintln!("Could not read file `{}`", filename);
            // Exit the program with exit code `1`.
            exit(1);
        }
    };

    println!("{}", contents);

    let config: Config = match toml::from_str(&contents) {
        // If successful, return data as `Data` struct.
        // `d` is a local variable.
        Ok(d) => d,
        // Handle the `error` case.
        Err(e) => {
            eprintln!("{}", e);
            // Write `msg` to `stderr`.
            eprintln!("Unable to load data from `{}`", filename);
            // Exit the program with exit code `1`.
            exit(1);
        }
    };

    return config;
}

#[tokio::main]
async fn main() -> Result<(), eyre::Report> {
    pretty_env_logger::init();

    let config = load_config("config.toml");

    let mut characteristic_readable: Option<CharacteristicInfo> = None;
    let mut characteristic_writable: Option<CharacteristicInfo> = None;
    let (_, session) = BluetoothSession::new().await?;

    println!("Session: {:#?}", session.get_adapters().await?);


    // Start scanning for Bluetooth devices, and wait a few seconds for some to be discovered.
    session.start_discovery().await?;
    time::sleep(Duration::from_secs(config.bluetooth.scan_duration.into())).await;
    session.stop_discovery().await?;

    // Get a list of devices which are currently known.
    let devices = session.get_devices().await?;

    println!("Devices: {:#?}", devices);

    // Find the device we care about.
    let device = match devices
        .into_iter()
        .find(|device| device.mac_address.to_string().ends_with(config.device.mac_address.as_str()))
    {
        Some(device) => device,
        None => {
            println!("Problem finding device. Aborting...");
            exit(1)
        }
    };

    // Connecting to device
    println!("Connecting to device {}: {}", device.mac_address, device.id);
    session.connect(&device.id).await?;

    // session.disconnect(&device.id).await?;
    // exit(1);

    // Get the list of devices whose services are currently known and print them with their
    // characteristics.
    println!("Getting service list...");
    let services = session.get_services(&device.id).await?;

    if !services.is_empty() {
        for service in services {
            println!("Service: {}: {}", service.uuid.succinctly(), service.id);
            let characteristics = session.get_characteristics(&service.id).await?;
            for characteristic in characteristics {
                // Find characteristic with READ flag set
                print!(
                    "Characteristic: {}: {:?} -> ",
                    characteristic.uuid.succinctly(),
                    characteristic.flags
                );
                if characteristic.flags.contains(CharacteristicFlags::READ) {
                    println!("setting as read source");
                    characteristic_readable = Some(characteristic.clone());
                    continue;
                }

                // Find characteristic with WRITE flag set
                if characteristic.flags.contains(CharacteristicFlags::WRITE) {
                    println!("setting as write source");
                    characteristic_writable = Some(characteristic.clone());
                    continue;
                }
            }
        }
    } else {
        println!("Services not found. Disconnecting...");
        session.disconnect(&device.id).await?;
        exit(1);
    }

    match characteristic_writable {
        Some(char) => {
            println!("Sending GET_DEVICE_INFO command to device...");
            // Write GET_DEVICE_INFO command to characteristic
            session
                .write_characteristic_value(
                    &char.id,
                    vec![
                        0xaa, 0x55, 0x90, 0xeb, 0x97, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11,
                    ],
                )
                .await?;

            // Wait a little bit
            time::sleep(Duration::from_secs(1)).await;

            println!("Sending GET_CELL_INFO command to device...");
            // Write GET_CELL_INFO command to characteristic
            session
                .write_characteristic_value(
                    &char.id,
                    vec![
                        0xaa, 0x55, 0x90, 0xeb, 0x96, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
                    ],
                )
                .await?;
        }
        None => {
            println!("Writable characteristic not found. Disconnecting...");
            session.disconnect(&device.id).await?;
            exit(1);
        }
    }

    match characteristic_readable {
        Some(char) => {
            let mut events = session.characteristic_event_stream(&char.id).await?;
            session.start_notify(&char.id).await?;
            println!("Waiting for notifications");

            let frame_size = 300;
            let mut full_frame: Vec<u8> = Vec::new();
            let mut wait_next_frame: bool = false;
            while let Some(event) = events.next().await {
                if let BluetoothEvent::Characteristic {
                    id: _,
                    event: CharacteristicEvent::Value { value },
                } = event
                {
                    if value[0] == 0xaa && value[1] == 0x55 {
                        continue;
                    }
                    if wait_next_frame {
                        full_frame.extend(value.clone());
                        wait_next_frame = false;
                    }
                    if value[0] == 0x55 && value[1] == 0xaa && value[2] == 0xeb && value[3] == 0x90
                    {
                        full_frame = value.clone();
                        wait_next_frame = true;
                    }

                    if full_frame.len() == frame_size {
                        let p = CellsInfo::new(full_frame.clone()).unwrap();
                        let zabbix = Sender::new(config.zabbix.address.as_str(), config.zabbix.port);
                        zabbix.send((config.zabbix.host.as_str(), config.zabbix.item.as_str(), to_string(&p)?.as_str()))?;
                        println!("Data: {:#?}", p);
                    }

                    // println!("Len: {}", full_frame.len());

                    time::sleep(Duration::from_secs((config.zabbix.send_timeout as f32 * 0.5) as u64)).await;
                } else {
                    println!("Other event {:?}", event)
                }
            }
        }
        None => {
            println!("Readable characteristic not found. Disconnecting");
            session.disconnect(&device.id).await?;
            exit(1);
        }
    }

    Ok(())
}
