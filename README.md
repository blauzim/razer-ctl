# Razer Blade control utility

This is a fork of the razer-ctl program that [tdakhran](https://github.com/tdakran) first created in 2024.  It has been updated to add support for the new Razer Blade 16 2025. It is still very much a work in progress and not all features have been tested on all models so yuour milage may vary. 

The supported devices are :
* Razer Blade 16 2025
* Razer Blade 16 2023
* Razer Blade 14 2023

## What can it control?

* Performance modes (including overclock & Hyperboost)
* Lid logo modes: off, static, breathing
* Keyboard brightness (works on Windows with Fn keys anyway)

![](data/demo.gif)

## What is missing vs ghelper?

* Windows power plan control
* Detecting and disabling / closing apps which use GPU when needed to save power
* GUI for fan controls
* Custom power targets for GPU and CPU - can't do as Razer interface doesn't support it.
* Detecting if power plugged in or not and changing the system settings (razer power settings, windows power plan, stop gpu apps,....)

## Reverse Engineering

Read about the reverse engineering process for Razer Blade 16 in [data/README.md](data/README.md). You can follow the steps and adjust the utility for other Razer laptops.

Run `razer-cli enumerate` to get PID.
Then `razer-cli -p 0xPID info` to check if the application works for your Razer device.

Special thanks to
* [tdakhran](https://github.com/tdakran) for the original code for this fork [repository](https://github.com/tdakhran/razer-ctl)
* [openrazer](https://github.com/openrazer) for [Reverse-Engineering-USB-Protocol](https://github.com/openrazer/openrazer/wiki/Reverse-Engineering-USB-Protocol)
* [Razer-Linux](https://github.com/Razer-Linux/razer-laptop-control-no-dkms) for USB HID protocol implementation

## FAQ

**Q**: *How to build?*

**A**: I build in WSL2(Arch) with `cargo run --release --target x86_64-pc-windows-gnu --bin razer-tray`.

**Q**: *Does it work on Linux?*

**A**: I didn't test, but nothing prevents it, all libraries are cross-platform.

**Q**: *Why Windows Defender tells me it is a Trojan*

**A**: Read https://github.com/rust-lang/rust/issues/88297, and make sure recent Intelligence Updates are installed for Microsoft Defender.

**Q**: *What's the easiest way to try?*

**A**: Download `razer-tray.exe` from [Releases](https://github.com/tdakhran/razer-ctl/releases) and launch it.
