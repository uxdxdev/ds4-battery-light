# ds4-battery-light

A tiny Windows utility that changes your DualShock 4's lightbar color as its battery drains, so you can see at a glance when it needs charging.

| State | Lightbar |
|---|---|
| Charging | Blue |
| Above 50% | Green |
| 21–50% | Orange |
| 20% or less | Red |

Works with DS4 v1 and v2 over USB or Bluetooth, and with several controllers at once.

## Usage

1. Download `ds4-battery-light.exe` from the [Releases](../../releases) page.
2. Close anything else that controls the lightbar (Steam, DS4Windows, etc.).
3. Run it. It runs in the background with no window and checks every 5 seconds.

A tray icon shows the current battery color (hover for the percentage). Right-click it and choose **Quit** to exit.

## Build from source

Requires [Rust](https://rustup.rs) and the Visual Studio C++ Build Tools.

```
cargo build --release
```

The binary ends up in `target/release/`.

## Customizing

Colors and thresholds are in `color_for` in `src/main.rs`; the check interval is `POLL_SECS`.

## Credits

The HID communication (battery parsing, Bluetooth output report CRC) is adapted from the `ds4-hid` crate in [symonxdd/ds4-dashboard](https://github.com/symonxdd/ds4-dashboard) (MIT).

## License

MIT
