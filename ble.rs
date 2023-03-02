use tokio::time;
use bluez_async::{
    BleUuid, BluetoothError, BluetoothEvent, BluetoothSession, CharacteristicEvent,
    CharacteristicFlags, CharacteristicInfo, DeviceInfo, ServiceInfo,
};
use futures::stream::StreamExt;
const SCAN_DURATION: Duration = Duration::from_secs(5);
use std::time::Duration;

pub struct BLE {
    pub session: BluetoothSession,
    pub device: DeviceInfo,
    pub services: Vec<ServiceInfo>,
    pub characteristics: Vec<CharacteristicInfo>,
    pub input: CharacteristicInfo,
    pub output: CharacteristicInfo,
}


impl BLE {
    pub async fn connect(mac_address: &str) -> Result<BLE, eyre::Report> {
        let (_, session) = BluetoothSession::new().await?;

        session.start_discovery().await?;
        time::sleep(SCAN_DURATION).await;
        session.stop_discovery().await?;

        // Get a list of devices which are currently known.
        let devices = session.get_devices().await?;

        // Find the device we care about.
        let device = match devices
            .into_iter()
            .find(|device| device.mac_address.to_string().ends_with(mac_address))
        {
            Some(device) => device,
            None => {
                return Err()
            }
        };

        match session.connect(&device.id).await {
            Ok() => _,
            Err(err) => return Err(err),
        };

        let services = match session.get_services(&device.id).await {
            Ok(services) => services,
            Err(err) => return Err(err),
        };

        let mut input: Option<CharacteristicInfo>;
        let mut output: Option<CharacteristicInfo>;

        if !services.is_empty() {
            for service in services {
                let characteristics = session.get_characteristics(&service.id).await?;
                for characteristic in characteristics {
                    if characteristic.flags.contains(CharacteristicFlags::READ) {
                        output = Some(characteristic.clone());
                        continue;
                    }
                    if characteristic.flags.contains(CharacteristicFlags::WRITE) {
                        input = Some(characteristic.clone());
                        continue;
                    }
                }
            }
        } else {
            return Err("Services not found. Disconnecting...");
        }

        Ok(BLE {
            session,
            device,
            services,
            characteristics,
            input: match input {
                Some(input) => input,
                None => return Err("Input not found"),
            },
            output: match output {
                Some(output) => output,
                None => return Err("Output not found"),
            },
        })
    }

    pub async fn get_device_info(&self) {
        self.session
            .write_characteristic_value(
                &self.input.id,
                vec![
                    0xaa, 0x55, 0x90, 0xeb, 0x97, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11,
                ],
            )
            .await?;
    }

    pub async fn get_cell_info(&self) {
        self.session
            .write_characteristic_value(
                &self.input.id,
                vec![
                    0xaa, 0x55, 0x90, 0xeb, 0x96, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
                ],
            )
            .await?;
    }

    pub async fn read_output(&self) -> Vec<u8> {
        let mut events = self
            .session
            .characteristic_event_stream(&self.output.id)
            .await?;
        self.session.start_notify(&self.output.id).await?;

        let frame_size = 300;
        let mut full_frame: Vec<u8> = Vec::new();
        let mut wait_next_frame: bool = false;

        while let Some(event) = events.next().await {
            if let BluetoothEvent::Characteristic {
                id,
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
                if value[0] == 0x55 && value[1] == 0xaa && value[2] == 0xeb && value[3] == 0x90 {
                    full_frame = value.clone();
                    wait_next_frame = true;
                }

                if full_frame.len() == frame_size {
                    let p = CellsInfo::new(full_frame.clone()).unwrap();
                    yield p
                    let zabbix = Sender::new("193.150.102.91", 10051);
                    zabbix.send(("JK_BMS", "jk.info", to_string(&p)?.as_str()))?;
                    println!("Data: {:#?}", p);
                }

                // println!("Len: {}", full_frame.len());

                time::sleep(Duration::from_secs((TIMEOUT * 0.5) as u64)).await;
            } else {
                println!("Other event {:?}", event)
            }
        }
    }
}

async fn connect(mac_address: &str) {}
