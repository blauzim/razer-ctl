use crate::feature;
use crate::types::PerfMode;

// model_number_prefix shall conform to https://mysupport.razer.com/app/answers/detail/a_id/5481
#[derive(Debug, Clone)]
pub struct Descriptor {
    pub model_number_prefix: &'static str,
    pub name: &'static str,
    pub pid: u16,
    pub features: &'static [&'static str],
    pub init_cmds: &'static [u16],
    /// Number of fan zones (2 for most models, 4 for Blade 17 2021)
    pub fan_zones: u8,
    /// Supported performance modes (None = all modes supported)
    pub perf_modes: Option<&'static [PerfMode]>,
}

pub const SUPPORTED: &[Descriptor] = &[
    Descriptor {
        model_number_prefix: "RZ09-0483T",
        name: "Razer Blade 16” (2023) Black",
        pid: 0x029f,
        features: &[
            "battery-care",
            "fan",
            "kbd-backlight",
            "lid-logo",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[],
        fan_zones: 2,
        perf_modes: None,  // All modes supported
    },
    Descriptor {
        model_number_prefix: "RZ09-0482X",
        name: "Razer Blade 14” (2023) Mercury",
        pid: 0x029d,
        features: &[
            "battery-care",
            "fan",
            "kbd-backlight",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[],
        fan_zones: 2,
        perf_modes: None,  // All modes supported
    },
    Descriptor {
        model_number_prefix: "RZ09-05289",
        name: "Razer Blade 16” (2025) 5090",
        pid: 0x02c6,
        features: &[
            "battery-care",
            "fan",
            "kbd-backlight",
            "lid-logo",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[0x0081, 0x0086, 0x0f90, 0x0086, 0x0f10, 0x0087],
        fan_zones: 2,
        perf_modes: None,  // All modes supported
    },
    Descriptor {
        model_number_prefix: "RZ09-05288",
        name: "Razer Blade 16” (2025) 5080",
        pid: 0x02c6,
        features: &[
            "battery-care",
            "fan",
            "kbd-backlight",
            "lid-logo",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[0x0081, 0x0086, 0x0f90, 0x0086, 0x0f10, 0x0087],
        fan_zones: 2,
        perf_modes: None,  // All modes supported
    },
    Descriptor {
        model_number_prefix: "RZ09-0421N",
        name: "Razer Blade 15” (2022)",
        pid: 0x028a,
        features: &[
            "battery-care",
            "fan",
            "kbd-backlight",
            "lid-logo",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[],
        fan_zones: 2,
        perf_modes: None,  // All modes supported
    },
    Descriptor {
        model_number_prefix: "RZ09-0406A",
        name: "Razer Blade 17\" (2021)",
        pid: 0x0279,
        features: &[
            // No battery-care on this model (not seen in Synapse captures)
            "fan",
            "kbd-backlight",
            "lid-logo",
            "lights-always-on",
            "perf",
        ],
        init_cmds: &[],
        fan_zones: 4,  // 4 zones (validated via Wireshark capture)
        perf_modes: Some(&[PerfMode::Balanced, PerfMode::Custom]),
    }
];

const _VALIDATE_FEATURES: () = {
    crate::const_for! { device in SUPPORTED => {
        feature::validate_features(device.features);
    }}
};
