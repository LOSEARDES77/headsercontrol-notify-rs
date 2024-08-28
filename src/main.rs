//! Headset Battery Notifier
//!
//! This program monitors the battery status of connected headsets and sends notifications
//! about their battery levels and connection status.

use std::{collections::HashMap, fmt::Display, process::Command, thread::sleep, time::Duration};

/// Configuration for the battery notifier
struct Config {
    /// Interval between polls in milliseconds
    polling_interval: u64,
    /// Enable debug output
    debug: bool,
    /// Battery level threshold for low battery notifications
    battery_threshold: u8,
}

/// Represents the current battery status of a device
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum BatteryStatus {
    Charging,
    Discharging,
    Disconnected,
}

/// Represents a connected device
#[derive(Clone)]
struct Device {
    /// Name of the device
    name: String,
    /// Current battery status
    battery_status: BatteryStatus,
    /// Current battery level (if available)
    battery: Option<u8>,
    /// Last battery level that triggered a notification
    last_notif_battery_level: Option<u8>,
}

impl Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Device: {} | Battery Status: {:?} | Battery: {:?} | Last Notif Battery Level: {:?}",
            self.name, self.battery_status, self.battery, self.last_notif_battery_level
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
        poll_devices(&config, &mut devices);
        sleep(Duration::from_millis(config.polling_interval));
    }
}

/// Poll connected devices and update their status
fn poll_devices(config: &Config, devices: &mut HashMap<String, Device>) {
    let hsc_output = get_headsetcontrol_output();
    let hsc_output_lines: Vec<&str> = hsc_output.split("Found").collect();

    for line in hsc_output_lines.iter().filter(|&l| !l.is_empty()) {
        if let Some(mut device) = parse_device(line) {
            update_device(config, devices, &mut device);
        }
    }

    if config.debug {
        for dev in devices.values() {
            println!("{}", dev);
        }
    }
}

/// Get the output from the headsetcontrol command
fn get_headsetcontrol_output() -> String {
    let hsc_output = Command::new("headsetcontrol")
        .arg("-b")
        .output()
        .expect("failed to execute process");
    String::from_utf8_lossy(&hsc_output.stdout).to_string()
}

/// Parse device information from a string
fn parse_device(device_str: &str) -> Option<Device> {
    let mut device = Device {
        name: String::new(),
        battery_status: BatteryStatus::Disconnected,
        battery: None,
        last_notif_battery_level: None,
    };

    for line in device_str.lines() {
        if line.contains("Status: BATTERY_AVAILABLE") {
            device.battery_status = BatteryStatus::Discharging;
        } else if line.contains("Status: BATTERY_CHARGING") {
            device.battery_status = BatteryStatus::Charging;
        } else if line.ends_with("!") && line.starts_with(" ") {
            device.name = line.trim().trim_end_matches('!').to_string();
        } else if line.contains("Level: ") {
            device.battery = line
                .trim()
                .replace("Level: ", "")
                .replace('%', "")
                .parse()
                .ok();
        }
    }

    if device.name.is_empty()
        || (device.battery_status == BatteryStatus::Disconnected && device.battery.is_none())
    {
        return None;
    }

    device.name = device
        .name
        .split('(')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    Some(device)
}

/// Update the device status and send notifications if necessary
fn update_device(config: &Config, devices: &mut HashMap<String, Device>, new_device: &mut Device) {
    if let Some(old_device) = devices.get(&new_device.name) {
        if old_device.last_notif_battery_level != new_device.last_notif_battery_level {
            return;
        }

        handle_device_status_change(old_device, new_device);
        handle_battery_level_change(config, old_device, new_device);
    } else {
        handle_new_device(new_device);
    }

    devices.insert(new_device.name.clone(), new_device.clone());
}

/// Handle changes in device connection status
fn handle_device_status_change(old_device: &Device, new_device: &mut Device) {
    if old_device.battery_status != BatteryStatus::Disconnected
        && new_device.battery_status == BatteryStatus::Disconnected
    {
        new_device.last_notif_battery_level = None;
        send_notification(&new_device.name, "Device disconnected", "battery-caution");
    } else if old_device.battery_status == BatteryStatus::Disconnected
        && new_device.battery_status != BatteryStatus::Disconnected
    {
        send_notification(&new_device.name, "New device connected", "battery");
        sleep(Duration::from_secs(1));
        if let Some(battery) = new_device.battery {
            new_device.last_notif_battery_level = Some(battery);
            send_notification(
                &new_device.name,
                &format!("Battery level: {}%", battery),
                "battery",
            );
        }
    }
}

/// Handle changes in battery level
fn handle_battery_level_change(config: &Config, old_device: &Device, new_device: &mut Device) {
    if let (Some(old_battery), Some(new_battery)) = (old_device.battery, new_device.battery) {
        if new_device.battery_status == BatteryStatus::Discharging && new_battery < old_battery {
            handle_discharging(config, new_device, new_battery);
        } else if new_device.battery_status == BatteryStatus::Charging && new_battery > old_battery
        {
            handle_charging(new_device, new_battery);
        }
    }
}

/// Handle notifications for discharging devices
fn handle_discharging(config: &Config, device: &mut Device, battery: u8) {
    if battery < config.battery_threshold {
        device.last_notif_battery_level = Some(battery);
        send_notification(
            &device.name,
            &format!("Battery level low: {}%", battery),
            "battery-low",
        );
    } else if battery % 5 == 0 {
        device.last_notif_battery_level = Some(battery);
        send_notification(
            &device.name,
            &format!("Battery level: {}%", battery),
            "battery",
        );
    }
}

/// Handle notifications for charging devices
fn handle_charging(device: &mut Device, battery: u8) {
    if battery == 100 {
        device.last_notif_battery_level = Some(battery);
        send_notification(
            &device.name,
            &format!("Battery level full: {}%", battery),
            "battery",
        );
    } else if battery % 5 == 0 {
        device.last_notif_battery_level = Some(battery);
        send_notification(&device.name, &format!("Charging {}%", battery), "battery");
    }
}

/// Handle notifications for newly connected devices
fn handle_new_device(device: &mut Device) {
    send_notification(&device.name, "New device connected", "battery");
    sleep(Duration::from_secs(1));
    if let Some(battery) = device.battery {
        device.last_notif_battery_level = Some(battery);
        send_notification(
            &device.name,
            &format!("Battery level: {}%", battery),
            "battery",
        );
    }
}

/// List of valid notification icons
const NOTIFICATION_ICONS: [&str; 4] = [
    "dialog-information",
    "battery-caution",
    "battery-low",
    "battery",
];

/// Send a desktop notification
fn send_notification(name: &str, content: &str, icon: &str) {
    let icon = if NOTIFICATION_ICONS.contains(&icon) {
        icon
    } else {
        "dialog-information"
    };

    let _ = Command::new("notify-send")
        .arg(name)
        .arg(content)
        .arg(format!("--icon={}", icon))
        .stdout(std::process::Stdio::null())
        .output()
        .expect("failed to execute process");
}
