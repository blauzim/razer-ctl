# Razer HID Protocol Research

**Date:** 2025-11-30  
**Researcher:** @robinwil  
**Device:** Razer Blade 17" (2021) - RZ09-0406A  
**Firmware:** v01.03 (from command 0x0081)

## Overview

This document captures findings from reverse-engineering the Razer Synapse USB HID protocol using Wireshark captures. The goal is to understand and document the commands used to control performance modes, fan settings, and other device features.

## Packet Structure

Each packet is 90 bytes with the following structure:

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | status | 0x00=New, 0x02=Success, 0x05=NotSupported |
| 1 | 1 | id | Random packet ID (for matching request/response) |
| 2-3 | 2 | remaining_packets | Usually 0x0000 |
| 4 | 1 | protocol_type | Usually 0x00 |
| 5 | 1 | data_size | Number of valid bytes in args |
| 6 | 1 | command_class | High byte of command |
| 7 | 1 | command_id | Low byte of command |
| 8-87 | 80 | args | Command arguments (padded with zeros) |
| 88 | 1 | crc | XOR of bytes 2-87 |
| 89 | 1 | reserved | Always 0x00 |

### CRC Calculation

The CRC is calculated as XOR of bytes 2-87 (inclusive). This was confirmed by matching Wireshark captures from Razer Synapse.

```rust
fn calculate_crc(packet_bytes: &[u8]) -> u8 {
    packet_bytes[2..88].iter().fold(0u8, |acc, &b| acc ^ b)
}
```

## Known Commands

### Device Information (Class 0x00)

| Command | Type | Args | Response | Description |
|---------|------|------|----------|-------------|
| 0x0081 | GET | `[0]` | `[1, 0, 0, 0, 48, 49, 48, 51, ...]` | Firmware version (ASCII "0103" = v1.03) |
| 0x0086 | GET | `[0]` | `[7]` | Unknown (possibly lighting zone count?) |
| 0x0087 | GET | `[0]` | `[1, 0, 1, 0, 48, 49, 48, 51, ...]` | Similar to 0x0081, also contains firmware version |

### Performance Control (Class 0x0D)

| Command | Type | Args | Response | Description |
|---------|------|------|----------|-------------|
| 0x0D02 | SET | `[0x01, zone, mode, fan_mode]` | Echo | Set performance mode per zone |
| 0x0D82 | GET | `[0, zone, 0, 0]` | `[0, zone, mode, fan_mode]` | Get performance mode per zone |
| 0x0D07 | SET | `[0x01, cluster, boost]` | Echo | Set CPU/GPU boost (requires Custom mode) |
| 0x0D87 | GET | `[0, cluster, 0]` | `[0, cluster, boost]` | Get CPU/GPU boost level |
| 0x0D01 | SET | `[0, zone, rpm/100]` | Echo | Set fan RPM (requires Manual fan mode) |
| 0x0D81 | GET | `[0, zone, 0]` | `[0, zone, rpm/100]` | Get target fan RPM |
| 0x0D88 | GET | `[0, zone, 0]` | `[0, zone, rpm/100]` | Get actual fan RPM |
| 0x0D89 | GET | `[0]` | `[2, 1]` | Fan zone info? (2 physical fans?) |
| 0x0D8B | GET | `[0]` | `[2]` | Unknown (returns 2) |
| 0x0D80 | GET | ??? | Timeout | Unknown (Synapse uses, we can't query) |
| 0x0D83 | GET | ??? | Timeout | Unknown (Synapse queries after set_perf_mode) |

#### Performance Mode Values

| Value | Mode | Notes |
|-------|------|-------|
| 0 | Balanced | Default mode |
| 4 | Custom | Used by Synapse for custom CPU/GPU boost |

**Note:** On Razer Blade 17" (2021), only Balanced (0) and Custom (4) modes are supported. Silent, Performance, Battery, and Hyperboost modes are not available on this model.

#### Fan Mode Values

| Value | Mode |
|-------|------|
| 0 | Auto |
| 1 | Manual |
| 4 | Auto (alternate?) |

#### Cluster Values

| Value | Cluster |
|-------|---------|
| 1 | CPU |
| 2 | GPU |

#### CPU Boost Values

| Value | Level |
|-------|-------|
| 0 | Low |
| 1 | Medium |
| 2 | High |
| 3 | Boost |

#### GPU Boost Values

| Value | Level |
|-------|-------|
| 0 | Low |
| 1 | Medium |
| 2 | High |

### Fan Speed Control (Class 0x07)

| Command | Type | Args | Response | Description |
|---------|------|------|----------|-------------|
| 0x070F | SET | `[mode]` | Echo | Set max fan speed mode |
| 0x078F | GET | `[0]` | `[mode]` | Get max fan speed mode |
| 0x078C | GET | ??? | Timeout | Unknown (Synapse uses) |

**Note:** Battery care commands (0x0712/0x0792) are NOT available on Razer Blade 17" (2021). These may be present on newer models like Blade 16.

### Lighting Control (Class 0x03)

| Command | Type | Args | Response | Description |
|---------|------|------|----------|-------------|
| 0x0300 | SET | `[1, 4, on/off]` | Echo | Set logo power |
| 0x0380 | GET | `[1, 4, 0]` | `[1, 4, on/off]` | Get logo power |
| 0x0302 | SET | `[1, 4, mode]` | Echo | Set logo mode (0=Static, 2=Breathing) |
| 0x0382 | GET | `[1, 4, 0]` | `[1, 4, mode]` | Get logo mode |
| 0x0303 | SET | `[1, 5, brightness]` | Echo | Set keyboard brightness (0-255) |
| 0x0383 | GET | `[1, 5, 0]` | `[1, 5, brightness]` | Get keyboard brightness |
| 0x030A | ??? | `[5, 0]` | ??? | Lighting commit/apply? |
| 0x030B | SET | `[0xFF, row, ...]` | ??? | RGB keyboard matrix row data |

### System Settings (Class 0x00)

| Command | Type | Args | Response | Description |
|---------|------|------|----------|-------------|
| 0x0004 | SET | `[on/off, 0]` | Echo | Set lights always on |
| 0x0084 | GET | `[0, 0]` | `[on/off, 0]` | Get lights always on |

## Synapse Boot Sequence

When Razer Synapse starts, it sends approximately 450 packets:

1. **Device queries** (0x0081, 0x0086, 0x0087) - Get firmware/device info
2. **Set Balanced mode** - Sets all 4 zones to Balanced/Auto
3. **Switch to Custom mode** - Sets all 4 zones to Custom/Auto
4. **Query capabilities** (0x0D83) - After each zone change
5. **Set RGB lighting** - 348 packets of keyboard matrix data (0x030B)
6. **Set lights_always_on** (0x0004)
7. **Apply saved boost settings** (0x0D07) - Only if previously configured

### Key Observation

Synapse **pushes saved settings** on startup rather than querying current device state. It does NOT use 0x0D82 (get perf_mode) or 0x0D87 (get boost) during normal boot.

## Zone Discovery

The Razer Blade 17" (2021) has **4 physical fans**:
- **Zones 1-2**: Two large fans at the top (near screen hinge), one on each side of the center
- **Zones 3-4**: Two smaller fans at the bottom center, side by side (appears as one unit but has 2 intakes)

Testing with manual fan RPM control:

| Zone | Actual RPM | Physical Location |
|------|------------|-------------------|
| 1 | ~2500 RPM | Top left (large) |
| 2 | ~2500 RPM | Top right (large) |
| 3 | ~2100 RPM | Bottom center (small) |
| 4 | 0 RPM | Bottom center (small) - may be reporting issue |

**Note:** All 4 fans were confirmed running during testing. Zone 4 reporting 0 RPM may be a firmware bug or query issue.

Command 0x0D89 returns `[2, 1]` - meaning unclear, possibly indicates 2 fan groups or configurations.

## Unknown/Undocumented Commands

These commands were seen in Synapse traffic but timeout when we try to query them:

- **0x0D80** - Large query (data_size=0x50), possibly thermal/power table?
- **0x0D83** - Sent after each set_perf_mode, possibly capabilities query
- **0x078C** - Unknown, class 0x07 (fan/power related?)

## Test Methodology

1. Capture USB traffic with Wireshark + USBPcap
2. Filter for HID SET_REPORT/GET_REPORT packets (I used filter `frame.len == 126`).
3. Extract Data Fragment from each packet
4. Decode using known packet structure
5. Test commands using `razer-cli auto cmd <command> <args...>`

## References

- [razer-laptop-control-no-dkms](https://github.com/Razer-Linux/razer-laptop-control-no-dkms)
- [OpenRazer](https://github.com/openrazer/openrazer)

## Changelog

- **2025-11-30**: Initial research, documented CRC calculation, discovered firmware query commands
