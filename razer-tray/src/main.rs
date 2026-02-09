#![windows_subsystem = "windows"]

use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use anyhow::Error;

use librazer::types::{BatteryCare, CpuBoost, GpuBoost, LightsAlwaysOn, LogoMode, FanMode};
use librazer::{command, device};

use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, PredefinedMenuItem, MenuItem, Submenu, MenuId},
    TrayIconBuilder, TrayIconEvent, 
};

use std::process::Command as procCommand;
use sysinfo::{ProcessExt, Signal, System, SystemExt};

use single_instance::SingleInstance;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{
    GetCurrentProcess, ProcessPowerThrottling, SetPriorityClass, SetProcessInformation,
    IDLE_PRIORITY_CLASS, PROCESS_POWER_THROTTLING_CURRENT_VERSION,
    PROCESS_POWER_THROTTLING_EXECUTION_SPEED, PROCESS_POWER_THROTTLING_STATE,
};
#[cfg(target_os = "windows")]
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum FanSpeed {
    Auto,
    Manual(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
enum PerfMode {
    Battery,
    Silent,
    Balanced,
    Performance,
    Hyperboost,
    Custom(CpuBoost, GpuBoost),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct LightsMode {
    logo_mode: LogoMode,
    keyboard_brightness: u8,
    always_on: LightsAlwaysOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct FanRpm {
    fan1: u16,
    fan2: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct DeviceState {
    perf_mode: PerfMode,
    lights_mode: LightsMode,
    battery_care: BatteryCare,
    fan_speed : FanSpeed,
}

type Result<T> = std::result::Result<T, Error>;

impl DeviceState {
    fn read(device: &device::Device) -> Result<Self> {
        let perf_mode = match command::get_perf_mode(device)? {
            (librazer::types::PerfMode::Battery, _) => PerfMode::Battery,
            (librazer::types::PerfMode::Silent, _) => PerfMode::Silent,
            (librazer::types::PerfMode::Balanced, _) => PerfMode::Balanced,
            (librazer::types::PerfMode::Performance, _) => PerfMode::Performance,
            (librazer::types::PerfMode::Hyperboost, _) => PerfMode::Hyperboost,
            (librazer::types::PerfMode::Custom, _) => {
                let cpu_boost = command::get_cpu_boost(device)?;
                let gpu_boost = command::get_gpu_boost(device)?;
                PerfMode::Custom(cpu_boost, gpu_boost)
            }
        };

        let fan_speed = match command::get_perf_mode(device)? {
            (_,FanMode::Auto) => FanSpeed::Auto,
            (_,FanMode::Manual) => {
                let rpm = command::get_fan_rpm(device, librazer::types::FanZone::Zone1)?;
                FanSpeed::Manual(rpm)
            }
        };
        let lights_mode = LightsMode {
            logo_mode: command::get_logo_mode(device)?,
            keyboard_brightness: command::get_keyboard_brightness(device)?,
            always_on: command::get_lights_always_on(device)?,
        };

        let battery_care = command::get_battery_care(device)?;

        Ok(Self {
            perf_mode,            
            lights_mode,
            battery_care,
            fan_speed
        })
    }

    fn apply(&self, device: &device::Device) -> Result<()> {
        match self.perf_mode {
            PerfMode::Battery => command::set_perf_mode(device, librazer::types::PerfMode::Battery),
            PerfMode::Silent => command::set_perf_mode(device, librazer::types::PerfMode::Silent),
            PerfMode::Balanced => command::set_perf_mode(device, librazer::types::PerfMode::Balanced),
            PerfMode::Performance => command::set_perf_mode(device, librazer::types::PerfMode::Performance),
            PerfMode::Hyperboost => command::set_perf_mode(device, librazer::types::PerfMode::Hyperboost),
            PerfMode::Custom(cpu_boost, gpu_boost) => {
                command::set_perf_mode(device, librazer::types::PerfMode::Custom)?;
                command::set_cpu_boost(device, cpu_boost)?;
                command::set_gpu_boost(device, gpu_boost)
            }
        }?;

        match self.fan_speed {
            FanSpeed::Auto => command::set_fan_mode(device, librazer::types::FanMode::Auto),
            FanSpeed::Manual(rpm) => {
                command::set_fan_mode(device, librazer::types::FanMode::Manual)?;
                command::set_fan_rpm(device, rpm, false)
            }
        }?;

        match self.lights_mode.logo_mode {
            LogoMode::Static => command::set_logo_mode(device, LogoMode::Static),
            LogoMode::Breathing => command::set_logo_mode(device, LogoMode::Breathing),
            LogoMode::Off => command::set_logo_mode(device, LogoMode::Off),
        }?;

        command::set_keyboard_brightness(device, self.lights_mode.keyboard_brightness)?;
        command::set_lights_always_on(device, self.lights_mode.always_on)?;
        command::set_battery_care(device, self.battery_care)
    }

    fn perf_delta(
        &self,
        cpu_boost: Option<CpuBoost>,
        gpu_boost: Option<GpuBoost>,
    ) -> Self {
        DeviceState {
            perf_mode: if let PerfMode::Custom(cb, gb) = self.perf_mode {
                PerfMode::Custom(
                    cpu_boost.unwrap_or(cb),
                    gpu_boost.unwrap_or(gb)
                )
            } else {
                PerfMode::Custom(
                    cpu_boost.unwrap_or(CpuBoost::Boost),
                    gpu_boost.unwrap_or(GpuBoost::High)
                )
            },
            ..*self
        }
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            perf_mode: PerfMode::Performance,
            lights_mode: LightsMode {
                logo_mode: LogoMode::Off,
                keyboard_brightness: 0,
                always_on: LightsAlwaysOn::Disable,
            },
            battery_care: BatteryCare::Percent80,
            fan_speed : FanSpeed::Auto,
        }
    }
}

trait DeviceStateDelta<T> {
    fn delta(&self, property: T) -> Self;
}

impl DeviceStateDelta<CpuBoost> for DeviceState {
    fn delta(&self, cpu_boost: CpuBoost) -> Self {
        self.perf_delta(Some(cpu_boost), None)
    }
}

impl DeviceStateDelta<GpuBoost> for DeviceState {
    fn delta(&self, gpu_boost: GpuBoost) -> Self {
        self.perf_delta(None, Some(gpu_boost))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct ConfigState {
    ac_state: DeviceState,
    battery_state: DeviceState,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            ac_state: DeviceState {..Default::default()},
            battery_state : DeviceState {
                    perf_mode : PerfMode::Battery,
                    ..Default::default()
                },
        }
    }
}


struct ProgramState {
    device_state: DeviceState,
    ac_state: DeviceState,
    battery_state: DeviceState,
    event_handlers: std::collections::HashMap<String, DeviceState>,
    menu: Menu,
    fan_actual : FanRpm,
    ac_power : bool
}

impl ProgramState {
    fn new(device_state: DeviceState, fan_last : FanRpm) -> Result<Self> {
        let (menu, event_handlers) = Self::create_menu_and_handlers(&device_state)?;
        let fan_actual = fan_last.clone();
        let ac_power = true;
        let ac_state = device_state.clone();
        let battery_state = device_state.clone();
        Ok(Self {
            device_state,
            ac_state,
            battery_state,
            event_handlers,
            menu,
            fan_actual,
            ac_power
        })
    }

    fn create_menu_and_handlers(
        dstate: &DeviceState,
    ) -> Result<(Menu, std::collections::HashMap<String, DeviceState>)> {
        let mut event_handlers = std::collections::HashMap::new();
        let menu = Menu::new();
        // header

        // perf
        let perf_modes = Submenu::new("Performance", true);
        // Battery
        perf_modes.append(&CheckMenuItem::with_id(
            format!("{:?}", PerfMode::Battery),
            "Battery",
            dstate.perf_mode != PerfMode::Battery,
            dstate.perf_mode == PerfMode::Battery,
            None,
        ))?;
        event_handlers.insert(
            format!("{:?}", PerfMode::Battery),
            DeviceState {
                perf_mode: PerfMode::Battery,
                ..*dstate
            },
        );
        // silent
        perf_modes.append(&CheckMenuItem::with_id(
            format!("{:?}", PerfMode::Silent),
            "Silent",
            dstate.perf_mode != PerfMode::Silent,
            dstate.perf_mode == PerfMode::Silent,
            None,
        ))?;
        event_handlers.insert(
            format!("{:?}", PerfMode::Silent),
            DeviceState {
                perf_mode: PerfMode::Silent,
                ..*dstate
            },
        );
        // balanced
        perf_modes.append(&CheckMenuItem::with_id(
            format!("{:?}", PerfMode::Balanced),
            "Balanced",
            dstate.perf_mode != PerfMode::Balanced,
            dstate.perf_mode == PerfMode::Balanced,
            None,
        ))?;
        event_handlers.insert(
            format!("{:?}", PerfMode::Balanced),
            DeviceState {
                perf_mode: PerfMode::Balanced,
                ..*dstate
            },
        );
        // performance
        perf_modes.append(&CheckMenuItem::with_id(
            format!("{:?}", PerfMode::Performance),
            "Performance",
            dstate.perf_mode != PerfMode::Performance,
            dstate.perf_mode == PerfMode::Performance,
            None,
        ))?;
        event_handlers.insert(
            format!("{:?}", PerfMode::Performance),
            DeviceState {
                perf_mode: PerfMode::Performance,
                ..*dstate
            },
        );
        // Hyperboost
        perf_modes.append(&CheckMenuItem::with_id(
            format!("{:?}", PerfMode::Hyperboost),
            "Hyperboost",
            dstate.perf_mode != PerfMode::Hyperboost,
            dstate.perf_mode == PerfMode::Hyperboost,
            None,
        ))?;
        event_handlers.insert(
            format!("{:?}", PerfMode::Hyperboost),
            DeviceState {
                perf_mode: PerfMode::Hyperboost,
                ..*dstate
            },
        );

        // custom
        let cpu_boosts: Vec<CheckMenuItem> = CpuBoost::iter()
            .map(|boost| {
                let event_id = format!("cpu_boost:{:?}", boost);
                event_handlers.insert(event_id.clone(), dstate.delta(boost));
                let checked = matches!(dstate.perf_mode, PerfMode::Custom(b, _) if b == boost);
                CheckMenuItem::with_id(event_id, format!("{:?}", boost), !checked, checked, None)
            })
            .collect();

        let gpu_boosts: Vec<CheckMenuItem> = GpuBoost::iter()
            .map(|boost| {
                let event_id = format!("gpu_boost:{:?}", boost);
                event_handlers.insert(event_id.clone(), dstate.delta(boost));
                let checked = matches!(dstate.perf_mode, PerfMode::Custom(_, b) if b == boost);
                CheckMenuItem::with_id(event_id, format!("{:?}", boost), !checked, checked, None)
            })
            .collect();

        let separator = PredefinedMenuItem::separator();

        perf_modes.append(&Submenu::with_items(
            "Custom",
            true,
            &cpu_boosts
                .iter()
                .map(|i| i as &dyn IsMenuItem)
                .chain([&separator as &dyn IsMenuItem])
                .chain(gpu_boosts.iter().map(|i| i as &dyn IsMenuItem))
                .collect::<Vec<_>>(),
        )?)?;

        menu.append(&perf_modes)?;

        // Fan Speed
        menu.append(&PredefinedMenuItem::separator())?;
        let fan_speeds: Vec<CheckMenuItem> = [CheckMenuItem::with_id(
            "fan_speeds:auto",
            "Fan: Auto",
            dstate.fan_speed != FanSpeed::Auto,
            dstate.fan_speed == FanSpeed::Auto,
            None,
        )]
        .into_iter()
        .chain((0..=5500).step_by(500).map(|rpm| {
            let event_id = format!("fan_speeds:{}", rpm);
            event_handlers.insert(
                event_id.clone(),
                DeviceState {
                    fan_speed: FanSpeed::Manual(rpm),
                    ..*dstate
                },
            );
            CheckMenuItem::with_id(
                event_id,
                format!("Fan: {} RPM", rpm),
                dstate.fan_speed != FanSpeed::Manual(rpm),
                dstate.fan_speed == FanSpeed::Manual(rpm),
                None,
            )
        }))
        .collect();
        event_handlers.insert(
            "fan_speeds:auto".to_string(),
            DeviceState {
                fan_speed: FanSpeed::Auto,
                ..*dstate
            },
        );

        menu.append(&Submenu::with_items(
            "Fan Speed",
            true,
            &fan_speeds
                .iter()
                .map(|i| i as &dyn IsMenuItem)
                .collect::<Vec<_>>(),
        )?)?;

        // logo
        menu.append(&PredefinedMenuItem::separator())?;
        let modes = LogoMode::iter()
            .map(|mode| {
                let event_id = format!("logo_mode:{:?}", mode);
                event_handlers.insert(
                    event_id.clone(),
                    DeviceState {
                        lights_mode: LightsMode {
                            logo_mode: mode,
                            ..dstate.lights_mode
                        },
                        ..*dstate
                    },
                );
                CheckMenuItem::with_id(
                    event_id,
                    format!("{:?}", mode),
                    dstate.lights_mode.logo_mode != mode,
                    dstate.lights_mode.logo_mode == mode,
                    None,
                )
            })
            .collect::<Vec<_>>();

        menu.append(&Submenu::with_items(
            "Logo",
            true,
            &modes
                .iter()
                .map(|i| i as &dyn IsMenuItem)
                .collect::<Vec<_>>(),
        )?)?;
        menu.append(&PredefinedMenuItem::separator())?;

        // lights always on
        menu.append(&CheckMenuItem::with_id(
            "lights_always_on",
            "Lights always on",
            true,
            dstate.lights_mode.always_on == LightsAlwaysOn::Enable,
            None,
        ))?;
        event_handlers.insert(
            "lights_always_on".to_string(),
            DeviceState {
                lights_mode: LightsMode {
                    always_on: match dstate.lights_mode.always_on {
                        LightsAlwaysOn::Enable => LightsAlwaysOn::Disable,
                        LightsAlwaysOn::Disable => LightsAlwaysOn::Enable,
                    },
                    ..dstate.lights_mode
                },
                ..*dstate
            },
        );

        let brightness_modes: Vec<CheckMenuItem> = (0..=100)
            .step_by(10)
            .map(|brightness| {
                let event_id = format!("brightness:{}", brightness);
                event_handlers.insert(
                    event_id.clone(),
                    DeviceState {
                        lights_mode: LightsMode {
                            keyboard_brightness: brightness / 2 * 5,
                            ..dstate.lights_mode
                        },
                        ..*dstate
                    },
                );
                CheckMenuItem::with_id(
                    event_id,
                    format!("Brightness: {}", brightness),
                    dstate.lights_mode.keyboard_brightness != brightness / 2 * 5,
                    dstate.lights_mode.keyboard_brightness == brightness / 2 * 5,
                    None,
                )
            })
            .collect();

        menu.append(&Submenu::with_items(
            "Brightness",
            true,
            &brightness_modes
                .iter()
                .map(|i| i as &dyn IsMenuItem)
                .collect::<Vec<_>>(),
        )?)?;

        // battery care submenu
        menu.append(&PredefinedMenuItem::separator())?;
        
        let battery_care_options = [
            (BatteryCare::Percent50, "50%", "battery_care_50"),
            (BatteryCare::Percent55, "55%", "battery_care_55"),
            (BatteryCare::Percent60, "60%", "battery_care_60"),
            (BatteryCare::Percent65, "65%", "battery_care_65"),
            (BatteryCare::Percent70, "70%", "battery_care_70"),
            (BatteryCare::Percent75, "75%", "battery_care_75"),
            (BatteryCare::Percent80, "80%", "battery_care_80"),
            (BatteryCare::Disable, "Disabled (100%)", "battery_care_disable"),
        ];
        
        let battery_care_items: Vec<CheckMenuItem> = battery_care_options
            .iter()
            .map(|(mode, label, id)| {
                event_handlers.insert(
                    id.to_string(),
                    DeviceState {
                        battery_care: *mode,
                        ..*dstate
                    },
                );
                CheckMenuItem::with_id(
                    id,
                    label,
                    true,
                    dstate.battery_care == *mode,
                    None,
                )
            })
            .collect();
        
        menu.append(&Submenu::with_items(
            "Battery Care",
            true,
            &battery_care_items
                .iter()
                .map(|i| i as &dyn IsMenuItem)
                .collect::<Vec<_>>(),
        )?)?;

        // gpu task killer
        menu.append(&PredefinedMenuItem::separator())?;
        let terminate_item = MenuItem::with_id("dgpu_terminate_proc","Terminate dGPU processes", true, None);
        menu.append(&terminate_item)?;
        // footer
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&PredefinedMenuItem::about(None, Some(Self::about())))?;
        menu.append(&PredefinedMenuItem::quit(None))?;

        Ok((menu, event_handlers))
    }

    fn handle_event(&self, event_id: &str) -> Result<DeviceState> {
        let next_state = self.event_handlers.get(event_id).ok_or(anyhow::anyhow!(
            "No event handler found for event_id: {}",
            event_id
        ))?;
        Ok(*next_state)
    }

    fn about() -> tray_icon::menu::AboutMetadata {
        tray_icon::menu::AboutMetadata {
            name: Some(PKG_NAME.into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            authors: Some(
                env!("CARGO_PKG_AUTHORS")
                    .split(';')
                    .map(|a| a.trim().to_string())
                    .collect::<Vec<_>>(),
            ),
            website: Some(format!(
                "{}\nLog: {}",
                env!("CARGO_PKG_HOMEPAGE"),
                get_logging_file_path().display()
            )),
            comments: Some(env!("CARGO_PKG_DESCRIPTION").into()),
            ..Default::default()
        }
    }

    fn get_next_perf_mode(&self) -> DeviceState {
        DeviceState {
            perf_mode: match self.device_state.perf_mode {
                PerfMode::Battery => PerfMode::Silent,
                PerfMode::Silent => PerfMode::Balanced,
                PerfMode::Balanced => PerfMode::Performance,
                PerfMode::Performance => PerfMode::Hyperboost,
                PerfMode::Hyperboost => {
                    PerfMode::Custom(CpuBoost::Boost, GpuBoost::High)
                }
                PerfMode::Custom(..) => PerfMode::Battery,
            },
            ..self.device_state
        }
    }

    fn tooltip(&self) -> Result<String> {
        use std::fmt::Write;
        let mut info = String::new();
        let mut status = String::new();

        match self.device_state.perf_mode {
            PerfMode::Battery => writeln!(&mut info, "Battery")?,
            PerfMode::Silent => writeln!(&mut info, "Silent")?,
            PerfMode::Balanced => writeln!(&mut info, "Balanced")?,
            PerfMode::Performance => writeln!(&mut info, "Performance")?,
            PerfMode::Hyperboost => writeln!(&mut info, "Hyperboost")?,
            PerfMode::Custom(cpu_boost, gpu_boost) => {
                writeln!(&mut info, "Custom",)?;
                writeln!(&mut info, "CPU: {:?}", cpu_boost)?;
                writeln!(&mut info, "GPU: {:?}", gpu_boost)?;
            }
        }
        match self.device_state.fan_speed {
            FanSpeed::Auto => writeln!(&mut info, "Fan Auto")?,
            FanSpeed::Manual(rpm) => writeln!(&mut info, "Fan {:?} RPM", rpm)?
        }
        
        writeln!(
            &mut info,
            "Fan actual : {:?}, {:?} PRM",
            self.fan_actual.fan1,
            self.fan_actual.fan2,
        )?;

        writeln!(
            &mut info,
            "Logo: {:?}",
            self.device_state.lights_mode.logo_mode
        )?;

        if self.device_state.lights_mode.keyboard_brightness > 0 {
            writeln!(
                &mut info,
                "🔆: {:?}",
                self.device_state.lights_mode.keyboard_brightness
            )?;
        }

        if self.device_state.lights_mode.always_on == LightsAlwaysOn::Enable {
            status.push('💡');
        }

        // Battery care with percentage
        match self.device_state.battery_care {
            BatteryCare::Disable => {}, // No indicator for disabled
            _ => {
                writeln!(
                    &mut info,
                    "🔋 Battery Care: {}%",
                    self.device_state.battery_care.to_percent()
                )?;
            }
        }

        Ok((info.to_string() + &status).trim_end().to_string())
    }

    fn icon(&self) -> tray_icon::Icon {
        let razer_red = include_bytes!("../icons/razer-red.png");
        let razer_blue = include_bytes!("../icons/razer-blue.png");
        let razer_brown = include_bytes!("../icons/razer-brown.png");
        let razer_yellow = include_bytes!("../icons/razer-yellow.png");
        let razer_green = include_bytes!("../icons/razer-green.png");
        let razer_violet = include_bytes!("../icons/razer-violet.png");

        let image = match self.device_state.perf_mode {
            PerfMode::Battery => image::load_from_memory(razer_blue),
            PerfMode::Silent => image::load_from_memory(razer_yellow),
            PerfMode::Balanced => image::load_from_memory(razer_green),
            PerfMode::Performance => image::load_from_memory(razer_red),
            PerfMode::Hyperboost => image::load_from_memory(razer_violet),
            PerfMode::Custom(_, _) => image::load_from_memory(razer_brown),
        };

        let (icon_rgba, icon_width, icon_height) = {
            let image = image.expect("Failed to open icon").into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    }

    fn update(
        &mut self,
        tray_icon: &mut tray_icon::TrayIcon,
        new_device_state: DeviceState,
        device: &device::Device
    ) -> Result<()> {
        self.device_state = new_device_state.clone();
        self.device_state.apply(device)?;
        (self.menu, self.event_handlers) = Self::create_menu_and_handlers(&self.device_state)?;
        self.fan_actual = get_fan_rpm(device)?;
        if self.ac_power {
            self.ac_state = self.device_state.clone()
        } else {
            self.battery_state = self.device_state.clone()
        }
        confy::store(PKG_NAME, None, &ConfigState {ac_state : self.ac_state,battery_state :  self.battery_state})?;
        tray_icon.set_icon(Some(self.icon()))?;
        tray_icon.set_tooltip(Some(self.tooltip()?))?;
        tray_icon.set_menu(Some(Box::new(self.menu.clone())));

        log::info!("state updated to {:?}", new_device_state);
        Ok(())
    }

}



#[cfg(target_os = "windows")]
fn get_power_state() -> Result<bool> {
    let mut ac_power : bool = true;
    unsafe {
        let mut status = SYSTEM_POWER_STATUS::default();
        match GetSystemPowerStatus(&mut status) {
            Ok(()) => {
                match status.ACLineStatus {
                    0 => ac_power = false,
                    _ => ac_power = true
                }
            }
            Err(e) => {
                eprintln!("Failed to get power status: {:?}", e);
            }
        }
    }
    Ok(ac_power)
}

#[cfg(target_os = "linux")]
fn get_power_state() -> Result<bool> {
    // Try AC adapter first
    if let Ok(online) = std::fs::read_to_string("/sys/class/power_supply/AC/online")
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/AC0/online"))
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/ACAD/online"))
    {
        return Ok(online.trim() == "1");
    }
    
    // Fallback: check battery status
    if let Ok(status) = std::fs::read_to_string("/sys/class/power_supply/BAT0/status")
        .or_else(|_| std::fs::read_to_string("/sys/class/power_supply/BAT1/status"))
    {
        let status = status.trim();
        return Ok(status == "Charging" || status == "Full" || status == "Not charging");
    }
    
    // Default to AC power if we can't detect
    log::warn!("Could not detect power state, assuming AC power");
    Ok(true)
}

fn get_fan_rpm(device: &device::Device) -> Result<FanRpm> {
    let fan_actual = FanRpm {
        fan1 : command::get_fan_actual_rpm(device, librazer::types::FanZone::Zone1)?,
        fan2 : command::get_fan_actual_rpm(device, librazer::types::FanZone::Zone2)?,
    };
    //log::info!("fans updated to {:?}", fan_actual);
    Ok(fan_actual)
}

#[cfg(target_os = "windows")]
fn gpu_taskkill() -> Result<()> {
    use std::os::windows::process::CommandExt;
    let whitelist: &[&str] = &["explorer.exe", "Insufficient Permissions"];

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let output = procCommand::new("nvidia-smi")
        .args(&["--query-compute-apps=name,pid", "--format=csv,noheader"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("Failed to execute nvidia-smi");

    if !output.status.success() {
        log::info!("nvidia-smi command failed or no GPU processes found");
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines = stdout.lines();

    let mut pids_to_kill = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            continue;
        }

        let name = parts[0];
        let pid: u32 = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        if whitelist.contains(&name) {
            log::info!("Skipping whitelisted process: {} ({})", pid, name);
        } else {
            pids_to_kill.push((pid, name.to_string()));
        }
    }

    if pids_to_kill.is_empty() {
        log::info!("No GPU-using processes to kill.");
        return Ok(());
    }

    let mut sys = System::new_all();
    sys.refresh_processes();

    for (pid, name) in pids_to_kill {
        if let Some(process) = sys.process(sysinfo::Pid::from(pid as usize)) {
            log::info!("Attempting to kill process {} ({})", pid, name);
            if process.kill_with(Signal::Kill).unwrap_or(false) {
                log::info!("Successfully killed PID {}", pid);
            } else {
                log::info!("Failed to kill PID {}", pid);
            }
        } else {
            log::info!("Process with PID {} not found", pid);
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn gpu_taskkill() -> Result<()> {
    // dGPU process termination for Linux
    let output = procCommand::new("nvidia-smi")
        .args(&["--query-compute-apps=name,pid", "--format=csv,noheader"])
        .output();
    
    if output.is_err() {
        log::info!("nvidia-smi not found or no GPU processes");
        return Ok(());
    }
    
    let output = output?;
    if !output.status.success() {
        return Ok(());
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut system = System::new_all();
    system.refresh_all();
    
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            continue;
        }
        
        let pid: usize = match parts[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        
        if let Some(process) = system.process(sysinfo::Pid::from(pid)) {
            log::info!("Terminating GPU process: {} (PID: {})", parts[0], pid);
            process.kill_with(Signal::Term);
        }
    }
    
    Ok(())
}


fn get_logging_file_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{}.log", PKG_NAME))
}

fn init_logging_to_file() -> Result<()> {
    use log4rs::append::rolling_file::policy::compound::{
        roll::delete::DeleteRoller, trigger::size::SizeTrigger, CompoundPolicy,
    };
    let policy = CompoundPolicy::new(
        Box::new(SizeTrigger::new(50 << 20)),
        Box::new(DeleteRoller::new()),
    );

    let logfile = log4rs::append::rolling_file::RollingFileAppender::builder()
        .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
            "{h({d(%Y-%m-%d %H:%M:%S)(local)} - {l}: {m}{n})}",
        )))
        .build(get_logging_file_path(), Box::new(policy))?;

    let config = log4rs::config::Config::builder()
        .appender(log4rs::config::Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            log4rs::config::Root::builder()
                .appender("logfile")
                .build(log::LevelFilter::Trace),
        )?;

    log4rs::init_config(config)?;
    Ok(())
}

fn init(tray_icon: &mut tray_icon::TrayIcon, device: &device::Device) -> Result<ProgramState> {
    log::info!(
        "loading config file {}",
        confy::get_configuration_file_path(PKG_NAME, None)?.display()
    );
    let config: ConfigState = confy::load(PKG_NAME, None).unwrap_or_default();
    let fan_actual = get_fan_rpm(device)?;
    let mut state = ProgramState::new(config.ac_state, fan_actual)?;
    state.ac_power = get_power_state()?;
    state.ac_state = config.ac_state.clone();
    state.battery_state = config.battery_state.clone();
    if state.ac_power == false {
        state.device_state = state.battery_state.clone()
    }
    state.update(tray_icon, state.device_state, device)?;
    Ok(state)
}

#[cfg(target_os = "windows")]
fn efficiency_mode() {
    unsafe {
        let handle: HANDLE = GetCurrentProcess();

        let _ = SetPriorityClass(handle, IDLE_PRIORITY_CLASS);

        let power_throttling = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        };
        let _ = SetProcessInformation(
            handle,
            ProcessPowerThrottling,
            &power_throttling as *const _ as *mut _,
            std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
        );
    }
}

fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Initialize GTK for tray icon on Linux
        gtk::init().map_err(|_| anyhow::anyhow!("Failed to initialize GTK"))?;
    }
    
    #[cfg(target_os = "windows")]
    efficiency_mode();

    // Create a named mutex (unique string for your app)
    let instance = SingleInstance::new("razer-tray").unwrap();
    if !instance.is_single() {
        println!("Another instance is already running. Exiting.");
        return Ok(());
    }

    init_logging_to_file()?;
    log::info!("{0} starting {1} {0}", "==".repeat(20), PKG_NAME);

    let device = match device::Device::detect() {
        Ok(d) => {
            log::info!(
                "detected device: {} (0x{:04X})",
                d.info().name,
                d.info().pid
            );
            d
        }
        Err(e) => {
            log::error!("{:?}", e);
            native_dialog::MessageDialog::new()
                .set_type(native_dialog::MessageType::Error)
                .set_text(format!("{:?}", e).as_str())
                .show_alert()?;
            return Err(e);
        }
    };

    let mut tray_icon = TrayIconBuilder::new().build()?;

    let mut state: ProgramState = init(&mut tray_icon, &device)?;

    let menu_channel = MenuEvent::receiver();
    let tray_channel = TrayIconEvent::receiver();
    let event_loop = EventLoopBuilder::new().build();

    let mut last_device_state_check_timestamp = std::time::Instant::now();

    // loop through the default start up sequence to initialise the device.
    for element in device.info().init_cmds {
        command::send_command(&device, *element, &[0,0,0,0])?;
    }

    event_loop.run(move |_, _, control_flow| {
        let now = std::time::Instant::now();
        *control_flow = ControlFlow::WaitUntil(now + std::time::Duration::from_millis(1000));

        if let Err(e) = (|| -> Result<()> {
            if let Ok(event) = menu_channel.try_recv() {
                log::info!("Menu Event {:?}", event.id);
                if event.id == MenuId("dgpu_terminate_proc".to_string()) {
                    log::info!("match event id");
                    gpu_taskkill()?;
                } else {
                    let new_device_state = state.handle_event(event.id.as_ref())?;
                    log::info!("new_device_state 1 {:?}", new_device_state);
                    state.update(&mut tray_icon, new_device_state, &device)?;
                }
            }

            if matches!(tray_channel.try_recv(), Ok(event) if event.click_type == tray_icon::ClickType::Left) {
                let new_device_state = state.get_next_perf_mode();
                log::info!("new_device_state 2 {:?}", new_device_state);
                state.update(&mut tray_icon, new_device_state, &device)?;
            }

            state.ac_power = get_power_state()?;
            if state.ac_power && state.device_state != state.ac_state {
                let new_device_state = state.ac_state.clone();
                log::info!("new_device_state 3 {:?}", new_device_state);
                state.update(&mut tray_icon, new_device_state, &device)?;
            } else if state.ac_power == false && state.device_state != state.battery_state {
                let new_device_state = state.battery_state.clone();
                log::info!("new_device_state 3 {:?}", new_device_state);
                state.update(&mut tray_icon, new_device_state, &device)?;
            } 

            if now > last_device_state_check_timestamp + std::time::Duration::from_secs(10)
            {
                last_device_state_check_timestamp = now;
                state.fan_actual =  get_fan_rpm(&device)?;
                let active_device_state = DeviceState::read(&device)?;
                if active_device_state != state.device_state {
                    log::warn!("overriding externally modified state {:?},",
                              active_device_state);
                    state.update(&mut tray_icon, state.device_state, &device)?;
               } else {
                    tray_icon.set_tooltip(Some(state.tooltip()?))?;
               }
            }

            Ok(())
        })() {
            log::error!("trying to recover from: {:?}", e);
            match init(&mut tray_icon, &device) {
                Ok(new_state) => {
                    state = new_state;
                },
                Err(e) => {
                    log::error!("failed to recover: {:?}", e);
                    *control_flow = ControlFlow::WaitUntil(now + std::time::Duration::from_secs(5));
                }
            }
        }
    })
}
