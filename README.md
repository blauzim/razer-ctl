# Razer Blade control utility

This is a fork of the razer-ctl program that [tdakhran](https://github.com/tdakran) first created in 2024.  It has been updated to add support for the new Razer Blade 16 2025. It is still very much a work in progress and not all features have been tested on all models so your milage may vary. 

The supported devices are :
* Razer Blade 16 2025 (RTX 5070/5080/5090)
* Razer Blade 16 2024
* Razer Blade 16 2023 (Black / RZ09-0483U)
* Razer Blade 15 2022
* Razer Blade 14 2023

## What can it control?

* Performance modes (including overclock & Hyperboost)
* **Battery care (charge limiting)** - 50%, 55%, 60%, 65%, 70%, 75%, 80%, or disabled (100%)
* Fan control (auto/manual with RPM settings)
* Lid logo modes: off, static, breathing
* Keyboard brightness (works on Windows with Fn keys anyway)
* Lights always-on toggle
* dGPU process termination (battery saving)

![](data/demo.gif)

## Battery Care Feature

The enhanced battery care system allows setting precise charge limits to optimize battery longevity:

**Available Options:** 50%, 55%, 60%, 65%, 70%, 75%, 80%, or disabled (100%)

**Usage:**
```bash
# CLI
./razer-cli auto battery-care set 50    # Set to 50% limit
./razer-cli auto battery-care get       # Check current setting

# Tray Menu
Right-click tray icon → Battery Care → Select percentage
```

**Compatibility:**
- ✅ **Verified:** Razer Blade 16 (2024), Razer Blade 16 (2023)
- 🔄 **Expected to work:** All models with battery-care feature (Blade 14/15/16 series)
- ℹ️  **Note:** Percentages match official Razer Synapse Battery Health Optimizer (50-80% range)

If you experience issues on your model, please report with your device model and PID.

## What is missing vs ghelper?

* Power settings seem to have no effect when AC power unplugged and on battery.
* Windows power plan control
* Detecting and disabling / closing apps which use GPU when needed to save power
* GUI for fan controls
* Custom power targets for GPU and CPU - can't do as Razer interface doesn't support it.

## Reverse Engineering

Read about the reverse engineering process for Razer Blade 16 in [data/README.md](data/README.md). You can follow the steps and adjust the utility for other Razer laptops.

Run `razer-cli enumerate` to get PID.
Then `razer-cli auto info` to check if the application works for your Razer device.
For unsupported models, try `razer-cli manual -p 0xPID info`.

Special thanks to
* [tdakhran](https://github.com/tdakran) for the original code for this fork [repository](https://github.com/tdakhran/razer-ctl)
* [openrazer](https://github.com/openrazer) for [Reverse-Engineering-USB-Protocol](https://github.com/openrazer/openrazer/wiki/Reverse-Engineering-USB-Protocol)
* [Razer-Linux](https://github.com/Razer-Linux/razer-laptop-control-no-dkms) for USB HID protocol implementation

## FAQ

**Q**: *How to build?*

**A**: Build natively with `cargo build --release --bin razer-cli` (or `--bin razer-tray`). Requires Rust toolchain. Cross-compilation from WSL2 also works with `--target x86_64-pc-windows-gnu`.

**Q**: *Does it work on Linux?*

**A**: Yes! Fully tested on Linux with native device detection, GTK tray support, and proper permissions via plugdev group.

**Q**: *Why Windows Defender tells me it is a Trojan*

**A**: Read https://github.com/rust-lang/rust/issues/88297, and make sure recent Intelligence Updates are installed for Microsoft Defender.

**Q**: *What's the easiest way to try?*

**A**: Download `razer-tray.exe` from [Releases](https://github.com/tdakhran/razer-ctl/releases) and launch it.
