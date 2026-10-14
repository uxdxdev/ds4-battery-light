//! Sets the DualShock 4 lightbar color from its battery level.
//! HID logic adapted from https://github.com/symonxdd/ds4-dashboard (crates/ds4-hid, MIT).
#![windows_subsystem = "windows"]

use crc::{Crc, CRC_32_ISO_HDLC};
use hidapi::{BusType, HidApi, HidDevice};
use std::{thread, time::Duration};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIconBuilder,
};

enum AppEvent {
    Battery { tooltip: String, colors: Vec<(u8, u8, u8)> },
    Menu(MenuEvent),
}

/// A filled circle split into equal vertical slices, one color per controller.
fn circle_icon(colors: &[(u8, u8, u8)]) -> Icon {
    const N: u32 = 32;
    let grey = [(128, 128, 128)];
    let colors = if colors.is_empty() { &grey[..] } else { colors };
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for y in 0..N {
        for x in 0..N {
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let a = if dx * dx + dy * dy <= 14.0 * 14.0 { 255 } else { 0 };
            let (r, g, b) = colors[(x as usize * colors.len() / N as usize).min(colors.len() - 1)];
            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }
    Icon::from_rgba(rgba, N, N).expect("icon")
}

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

fn color_for(percent: u32) -> (u8, u8, u8) {
    if percent > 50 {
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

fn poll_loop(proxy: tao::event_loop::EventLoopProxy<AppEvent>) {
    let mut api = HidApi::new().expect("failed to init HID");
    loop {
        let _ = api.refresh_devices();
        let mut found: Vec<(u32, bool)> = Vec::new();
        for info in api.device_list() {
            if !DS4_IDS.contains(&(info.vendor_id(), info.product_id())) {
                continue;
            }
            let Ok(dev) = info.open_device(&api) else { continue };
            let bluetooth = matches!(info.bus_type(), BusType::Bluetooth);
            if let Some((pct, charging)) = read_battery(&dev, bluetooth) {
                set_lightbar(&dev, bluetooth, color_for(pct));
                found.push((pct, charging));
            }
        }
        let colors: Vec<_> = found.iter().map(|&(pct, _)| color_for(pct)).collect();
        let tooltip = if found.is_empty() {
            "DS4 battery light: no controller".to_string()
        } else {
            let lines: Vec<String> = found
                .iter()
                .enumerate()
                .map(|(i, (p, c))| {
                    format!("Controller {}: {p}%{}", i + 1, if *c { " (charging)" } else { "" })
                })
                .collect();
            // Windows tray tooltips are limited to 127 characters.
            lines.join("\n").chars().take(127).collect()
        };
        let _ = proxy.send_event(AppEvent::Battery { tooltip, colors });
        thread::sleep(Duration::from_secs(POLL_SECS));
    }
}

fn main() {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(AppEvent::Menu(e));
    }));

    let quit = MenuItem::new("Quit", true, None);
    let menu = Menu::new();
    menu.append(&quit).expect("menu");
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("DS4 battery light")
        .with_icon(circle_icon(&[]))
        .build()
        .expect("tray icon");

    let proxy = event_loop.create_proxy();
    thread::spawn(move || poll_loop(proxy));

    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        if let tao::event::Event::UserEvent(ev) = event {
            match ev {
                AppEvent::Battery { tooltip, colors } => {
                    let _ = tray.set_tooltip(Some(tooltip));
                    let _ = tray.set_icon(Some(circle_icon(&colors)));
                }
                AppEvent::Menu(e) if e.id == *quit.id() => *flow = ControlFlow::Exit,
                AppEvent::Menu(_) => {}
            }
        }
    });
}
