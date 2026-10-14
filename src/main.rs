//! Sets the DualShock 4 lightbar color from its battery level.
//! HID logic adapted from https://github.com/symonxdd/ds4-dashboard (crates/ds4-hid, MIT).

use crc::{Crc, CRC_32_ISO_HDLC};
use hidapi::{BusType, HidApi, HidDevice};
use std::{thread, time::Duration};

const DS4_IDS: [(u16, u16); 4] = [
    (0x054C, 0x05C4),
    (0x054C, 0x09CC),
    (0x054C, 0x0BA0),
    (0x054C, 0x05C5),
];
const POLL_SECS: u64 = 5;

/// Returns (battery percent, charging).
fn read_battery(dev: &HidDevice, bluetooth: bool) -> Option<(u32, bool)> {
    // A Bluetooth DS4 starts in basic mode (no battery data) until this feature report is read.
    if bluetooth {
        let mut buf = [0u8; 64];
        buf[0] = 0x02;
        let _ = dev.get_feature_report(&mut buf);
    }
    let (id, offset) = if bluetooth { (0x11, 3) } else { (0x01, 1) };
    let mut buf = [0u8; 256];
    for _ in 0..40 {
        let n = dev.read_timeout(&mut buf, 50).ok()?;
        if n == 0 || buf[0] != id || n < offset + 37 {
            continue;
        }
        let b = buf[offset + 29];
        let charging = b & 0x10 != 0;
        let raw = (b & 0x0F) as u32;
        let full = if charging { 11 } else { 8 };
        return Some(((raw * 100 / full).min(100), charging));
    }
    None
}

fn color_for(percent: u32, charging: bool) -> (u8, u8, u8) {
    if charging {
        (0, 0, 255) // blue
    } else if percent > 50 {
        (0, 255, 0) // green
    } else if percent > 20 {
        (255, 140, 0) // orange
    } else {
        (255, 0, 0) // red
    }
}

fn set_lightbar(dev: &HidDevice, bluetooth: bool, (r, g, b): (u8, u8, u8)) {
    if bluetooth {
        let mut rep = [0u8; 78];
        rep[0] = 0x11;
        rep[1] = 0x80;
        rep[3] = 0xFF;
        rep[8] = r;
        rep[9] = g;
        rep[10] = b;
        // CRC covers a 0xA2 prefix (not sent) plus the first 74 bytes.
        let mut crc_in = [0u8; 75];
        crc_in[0] = 0xA2;
        crc_in[1..].copy_from_slice(&rep[..74]);
        let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(&crc_in);
        rep[74..].copy_from_slice(&crc.to_le_bytes());
        let _ = dev.write(&rep);
    } else {
        let mut rep = [0u8; 32];
        rep[0] = 0x05;
        rep[1] = 0xFF;
        rep[6] = r;
        rep[7] = g;
        rep[8] = b;
        let _ = dev.write(&rep);
    }
}

fn main() {
    let mut api = HidApi::new().expect("failed to init HID");
    println!("ds4-battery-light running (checks every {POLL_SECS}s). Ctrl+C to quit.");
    loop {
        let _ = api.refresh_devices();
        for info in api.device_list() {
            if !DS4_IDS.contains(&(info.vendor_id(), info.product_id())) {
                continue;
            }
            let Ok(dev) = info.open_device(&api) else { continue };
            let bluetooth = matches!(info.bus_type(), BusType::Bluetooth);
            if let Some((pct, charging)) = read_battery(&dev, bluetooth) {
                println!("battery {pct}%{}", if charging { " (charging)" } else { "" });
                set_lightbar(&dev, bluetooth, color_for(pct, charging));
            }
        }
        thread::sleep(Duration::from_secs(POLL_SECS));
    }
}
