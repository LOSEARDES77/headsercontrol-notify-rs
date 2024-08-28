use std::{collections::HashMap, fmt::Display, process::Command, thread::sleep, time::Duration};

struct Config {
    polling_interval: u64,
    debug: bool,
    battery_threshold: u8,
}

#[derive(Debug, PartialEq, Eq)]
enum BatteryStatus {
    Charging,
    Discharging,
    Disconnected,
}

struct Device {
    name: String,
    battery_status: BatteryStatus,
    battery: Option<u8>,
    last_notif_battery_level: Option<u8>,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Device: {} | Battery Status: {:?} | Battery: {:?}",
            self.name, self.battery_status, self.battery
        )
    }
}

fn main() {
    println!("Starting Headset Battery Notifier...");
    let config = Config {
        polling_interval: 5000,
        debug: true,
        battery_threshold: 10,
    };

    let mut devices: HashMap<String, Device> = HashMap::new();

    loop {
        let start_polling = std::time::Instant::now();

        let hsc_output = Command::new("headsetcontrol")
            .arg("-b")
            .output()
            .expect("failed to execute process");

        let hsc_output_str = String::from_utf8_lossy(&hsc_output.stdout);

        let hsc_output_lines: Vec<&str> = hsc_output_str.split("Found").collect();
        for line in hsc_output_lines.iter() {
            if line.is_empty() {
                continue;
            }

            let mut device = match parse_device(line) {
                Some(device) => {
                    println!("Device found");
                    device
                }
                None => continue,
            };

            if let Some(old_device) = devices.get(&device.name) {
                if old_device.last_notif_battery_level != device.last_notif_battery_level {
                    continue;
                }
                println!("Device found in devices list");
                if old_device.battery_status != BatteryStatus::Disconnected
                    && device.battery_status == BatteryStatus::Disconnected
                {
                    device.last_notif_battery_level = None;
                    println!("Device disconnected");
                    send_notification(&device.name, "Device disconnected", "battery-caution");
                } else if let Some(battery) = device.battery {
                    if let Some(old_battery) = old_device.battery {
                        if device.battery_status == BatteryStatus::Discharging
                            && battery < old_battery
                        {
                            if battery < config.battery_threshold {
                                device.last_notif_battery_level = Some(battery);
                                println!("Battery level low");
                                send_notification(
                                    &device.name,
                                    &format!("Battery level low: {}%", battery),
                                    "battery-low",
                                );
                            } else if battery % 5 == 0 {
                                device.last_notif_battery_level = Some(battery);
                                println!("Battery level changed");
                                send_notification(
                                    &device.name,
                                    &format!("Battery level: {}%", battery),
                                    "battery",
                                );
                            }
                        } else if device.battery_status == BatteryStatus::Charging
                            && battery > old_battery
                        {
                            if battery == 100 {
                                device.last_notif_battery_level = Some(battery);
                                println!("Battery level full");
                                send_notification(
                                    &device.name,
                                    &format!("Battery level full: {}%", battery),
                                    "battery",
                                );
                            } else if battery % 5 == 0 {
                                device.last_notif_battery_level = Some(battery);
                                println!("Battery level increased");
                                send_notification(
                                    &device.name,
                                    &format!("Charging {}%", battery),
                                    "battery",
                                );
                            }
                        }
                    }
                } else if old_device.battery_status == BatteryStatus::Disconnected
                    && device.battery_status != BatteryStatus::Disconnected
                {
                    println!("New device connected");
                    send_notification(&device.name, "New device connected", "battery");
                    sleep(Duration::from_millis(1000));
                    send_notification(
                        &device.name,
                        &format!("Battery level: {}%", device.battery.unwrap()),
                        "battery",
                    );
                }
            }

            devices.insert(device.name.clone(), device);
        }

        if config.debug {
            for dev in devices.values() {
                println!("{}", dev);
            }
        }

        sleep(Duration::from_millis(config.polling_interval) - start_polling.elapsed());
    }
}

fn parse_device(device_str: &str) -> Option<Device> {
    let mut device = Device {
        name: String::new(),
        battery_status: BatteryStatus::Disconnected,
        battery: None,
        last_notif_battery_level: None,
    };

    let device_str_lines: Vec<&str> = device_str.split("\n").collect();
    for line in device_str_lines.iter() {
        if line.contains("Status: BATTERY_AVAILABLE") {
            device.battery_status = BatteryStatus::Discharging;
        } else if line.contains("Status: BATTERY_CHARGING") {
            device.battery_status = BatteryStatus::Charging;
        } else if line.ends_with("!") && line.starts_with(" ") {
            device.name = line.trim().to_string().replace("!", "");
        } else if line.contains("Level: ") {
            device.battery = Some(
                line.trim()
                    .replace("Level: ", "")
                    .replace("%", "")
                    .parse()
                    .unwrap(),
            );
        }
    }

    if device.name.is_empty()
        || device.battery_status == BatteryStatus::Disconnected && device.battery.is_none()
    {
        return None;
    }

    device.name = device.name.split("(").collect::<Vec<&str>>()[0]
        .trim()
        .to_string();

    Some(device)
}

const NOTIFICATION_ICONS: [&str; 9] = [
    "dialog-information",
    "dialog-warning",
    "dialog-error",
    "dialog-question",
    "dialog-password",
    "dialog-chat",
    "battery-caution",
    "battery-low",
    "battery",
];

fn send_notification(name: &str, content: &str, icon: &str) {
    let mut icon = icon;
    if !NOTIFICATION_ICONS.contains(&icon) {
        icon = "dialog-information";
    }

    let _ = Command::new("notify-send")
        .arg(name)
        .arg(content)
        .arg(format!("--icon={}", icon))
        .stdout(std::process::Stdio::null())
        .output()
        .expect("failed to execute process");
}
