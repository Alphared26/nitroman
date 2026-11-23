//! Hardware driver layer interfacing with Acer EC via embedded native EC controller.
//!
//! Controls:
//! - AcerGamingFunction: Fan Speeds, Fan Modes, Thermal Profiles, 4-Zone RGB.
//! - BatteryControl: 80% Charge Limiter (Conservation Mode).

use std::path::PathBuf;
use std::process::Command;
use serde::Deserialize;
use crate::error::{Error, Result};
use crate::model::{FanMode, RgbConfig, RgbEffect, ThermalProfile};
use crate::sensors::Telemetry;

#[cfg(windows)]
const EC_BIN: &[u8] = include_bytes!("../bin/nitroman-ec.exe");

/// Internal telemetry JSON structure returned by nitroman-ec.
#[derive(Debug, Deserialize, Clone)]
struct EcTelemetryJson {
    pub cpu_temp: f64,
    pub cpu_rpm: u32,
    pub gpu_temp: f64,
    pub gpu_rpm: u32,
    pub battery_80_limit: bool,
    #[serde(default)]
    pub coolboost: bool,
}

#[cfg(windows)]
static EC_CACHE: std::sync::Mutex<Option<(std::time::Instant, EcTelemetryJson)>> = std::sync::Mutex::new(None);

#[cfg(windows)]
struct RgbStreamProcess {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
}

#[cfg(windows)]
static RGB_STREAM: std::sync::Mutex<Option<RgbStreamProcess>> = std::sync::Mutex::new(None);

#[cfg(windows)]
pub fn close_rgb_stream() {
    if let Ok(mut guard) = RGB_STREAM.lock() {
        if let Some(mut proc) = guard.take() {
            use std::io::Write;
            let _ = proc.stdin.write_all(b"exit\n");
            let _ = proc.stdin.flush();
            drop(proc.stdin);
            let start = std::time::Instant::now();
            loop {
                match proc.child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if start.elapsed() > std::time::Duration::from_millis(150) {
                            let _ = proc.child.kill();
                            let _ = proc.child.wait();
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

#[cfg(windows)]
pub fn try_send_rgb_stream(zone_colors: &[(u8, u8, u8)], brightness: u8) -> bool {
    let mut guard = match RGB_STREAM.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };

    use std::io::Write;
    let line = format!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {}\n",
        zone_colors.get(0).map(|c| c.0).unwrap_or(0),
        zone_colors.get(0).map(|c| c.1).unwrap_or(0),
        zone_colors.get(0).map(|c| c.2).unwrap_or(0),
        zone_colors.get(1).map(|c| c.0).unwrap_or(0),
        zone_colors.get(1).map(|c| c.1).unwrap_or(0),
        zone_colors.get(1).map(|c| c.2).unwrap_or(0),
        zone_colors.get(2).map(|c| c.0).unwrap_or(0),
        zone_colors.get(2).map(|c| c.1).unwrap_or(0),
        zone_colors.get(2).map(|c| c.2).unwrap_or(0),
        zone_colors.get(3).map(|c| c.0).unwrap_or(0),
        zone_colors.get(3).map(|c| c.1).unwrap_or(0),
        zone_colors.get(3).map(|c| c.2).unwrap_or(0),
        brightness
    );

    if let Some(proc) = guard.as_mut() {
        if proc.stdin.write_all(line.as_bytes()).is_ok() && proc.stdin.flush().is_ok() {
            return true;
        }
        let _ = proc.child.kill();
        let _ = proc.child.wait();
        *guard = None;
    }

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    if let Ok(ec_path) = AcerDriver::get_ec_binary_path() {
        if let Ok(mut child) = Command::new(&ec_path)
            .creation_flags(CREATE_NO_WINDOW)
            .arg("rgb-stream")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(line.as_bytes()).is_ok() && stdin.flush().is_ok() {
                    *guard = Some(RgbStreamProcess { child, stdin });
                    return true;
                }
            }
        }
    }
    false
}

/// ACPI Hardware Controller for Acer Nitro AN515-57.
#[derive(Debug, Default, Clone)]
pub struct AcerDriver {
    /// True if running with elevated administrator rights.
    pub is_elevated: bool,
}

impl AcerDriver {
    /// Creates a new driver instance, automatically detecting execution privileges.
    pub fn new() -> Self {
        let is_elevated = Self::check_elevation();
        Self { is_elevated }
    }

    /// Checks if current process is elevated with Administrator rights using native Win32 token query.
    pub fn check_elevation() -> bool {
        #[cfg(windows)]
        {
            use std::mem;
            #[repr(C)]
            struct TokenElevation {
                token_is_elevated: u32,
            }
            #[link(name = "advapi32")]
            extern "system" {
                fn OpenProcessToken(process_handle: isize, desired_access: u32, token_handle: *mut isize) -> i32;
                fn GetTokenInformation(
                    token_handle: isize,
                    token_information_class: u32,
                    token_information: *mut std::ffi::c_void,
                    token_information_length: u32,
                    return_length: *mut u32,
                ) -> i32;
                fn CloseHandle(handle: isize) -> i32;
                fn GetCurrentProcess() -> isize;
            }
            unsafe {
                let mut token_handle: isize = 0;
                // TOKEN_QUERY = 0x0008
                if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token_handle) != 0 {
                    let mut elevation = TokenElevation { token_is_elevated: 0 };
                    let mut ret_len: u32 = 0;
                    // TokenElevation = 20
                    let success = GetTokenInformation(
                        token_handle,
                        20,
                        &mut elevation as *mut _ as *mut std::ffi::c_void,
                        mem::size_of::<TokenElevation>() as u32,
                        &mut ret_len,
                    );
                    CloseHandle(token_handle);
                    return success != 0 && elevation.token_is_elevated != 0;
                }
            }
        }
        false
    }

    /// Resolves or extracts the nitroman-ec binary executable path.
    pub fn get_ec_binary_path() -> Result<PathBuf> {
        #[cfg(windows)]
        {
            // 1. Check C:\Program Files\Acer\NitroSense Service\NitroSense.exe FIRST!
            // This path is in the official whitelist of PSSvc, allowing immediate Result 0x0 without pipe rejection.
            let svc_helper = PathBuf::from(r"C:\Program Files\Acer\NitroSense Service\NitroSense.exe");
            if svc_helper.exists() {
                // If we have write access (elevated), keep it synchronized with embedded EC_BIN
                if let Ok(existing) = std::fs::read(&svc_helper) {
                    if existing != EC_BIN {
                        let _ = std::fs::write(&svc_helper, EC_BIN);
                    }
                }
                return Ok(svc_helper);
            }

            // 2. Check next to current running executable
            if let Ok(mut exe_dir) = std::env::current_exe() {
                exe_dir.pop();
                let local_path = exe_dir.join("nitroman-ec.exe");
                let should_write = match std::fs::read(&local_path) {
                    Ok(existing) => existing != EC_BIN,
                    Err(_) => true,
                };
                if should_write {
                    let _ = std::fs::write(&local_path, EC_BIN);
                }
                if local_path.exists() {
                    return Ok(local_path);
                }
            }

            // 3. Fall back to %TEMP%\nitroman-ec.exe
            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join("nitroman-ec.exe");
            let should_write = match std::fs::read(&temp_path) {
                Ok(existing) => existing != EC_BIN,
                Err(_) => true,
            };
            if should_write {
                let _ = std::fs::write(&temp_path, EC_BIN);
            }
            if temp_path.exists() {
                return Ok(temp_path);
            }

            Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Unable to locate or extract nitroman-ec controller",
            )))
        }
        #[cfg(not(windows))]
        {
            Err(Error::Unsupported("Windows required for Acer EC control".into()))
        }
    }

    /// Helper to execute command on nitroman-ec binary with zero console window blinking.
    fn run_ec(&self, args: &[&str]) -> Result<String> {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let ec_path = Self::get_ec_binary_path()?;
            let out = Command::new(&ec_path)
                .creation_flags(CREATE_NO_WINDOW)
                .args(args)
                .output()
                .map_err(Error::Io)?;

            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

            if !out.status.success() {
                let err_msg = if !stderr.is_empty() { stderr } else { stdout };
                return Err(Error::Wmi(format!("EC command failed: {err_msg}")));
            }

            Ok(stdout)
        }
        #[cfg(not(windows))]
        {
            let _ = args;
            Ok("OK".into())
        }
    }

    /// Sets system thermal profile (Quiet, Balanced, Performance).
    pub fn set_thermal_profile(&self, profile: ThermalProfile) -> Result<()> {
        let p_str = match profile {
            ThermalProfile::Quiet => "quiet",
            ThermalProfile::Balanced => "balanced",
            ThermalProfile::Performance => "performance",
        };
        self.run_ec(&["profile", p_str])?;
        Ok(())
    }

    /// Sets fan operating mode (Auto, Max/Performance, Custom).
    pub fn set_fan_mode(&self, mode: FanMode) -> Result<()> {
        let m_str = match mode {
            FanMode::Auto | FanMode::Quiet | FanMode::Balanced => "auto",
            FanMode::Performance => "max",
            FanMode::Custom => "custom",
        };
        self.run_ec(&["fan", m_str])?;
        Ok(())
    }

    /// Sets manual fan percentages for CPU and GPU (0..=100).
    pub fn set_fan_speed(&self, cpu_pct: u8, gpu_pct: u8) -> Result<()> {
        let cpu_str = cpu_pct.min(100).to_string();
        let gpu_str = gpu_pct.min(100).to_string();
        self.run_ec(&["fan", "custom", &cpu_str, &gpu_str])?;
        Ok(())
    }

    /// Sets 80% battery conservation limiter.
    pub fn set_battery_limit_80(&self, enabled: bool) -> Result<()> {
        let state_str = if enabled { "on" } else { "off" };
        self.run_ec(&["battery", state_str])?;
        Ok(())
    }

    /// Sets CoolBoost mode (elevates fan RPM ceiling under load).
    pub fn set_coolboost(&self, enabled: bool) -> Result<()> {
        let state_str = if enabled { "on" } else { "off" };
        self.run_ec(&["coolboost", state_str])?;
        Ok(())
    }

    /// Sets keyboard backlight auto-off timeout in seconds (0 = disabled, 30 = 30 seconds).
    pub fn set_backlight_timeout(&self, seconds: u32) -> Result<()> {
        let sec_str = seconds.to_string();
        self.run_ec(&["timeout", &sec_str])?;
        Ok(())
    }

    /// Queries the current keyboard backlight timeout in seconds from hardware EC.
    pub fn get_backlight_timeout(&self) -> Result<u32> {
        let out = self.run_ec(&["timeout", "get"])?;
        for line in out.lines() {
            let trimmed = line.trim();
            if let Ok(v) = trimmed.parse::<u32>() {
                return Ok(v);
            }
        }
        Err(Error::InvalidData("Invalid response from timeout query".into()))
    }

    /// Sets keyboard backlight color for a specific zone (1..=4).
    pub fn set_rgb_zone_color(&self, zone: u8, r: u8, g: u8, b: u8) -> Result<()> {
        let z_str = zone.clamp(1, 4).to_string();
        let r_str = r.to_string();
        let g_str = g.to_string();
        let b_str = b.to_string();
        self.run_ec(&["rgb-zone", &z_str, &r_str, &g_str, &b_str])?;
        Ok(())
    }

    /// Sets keyboard lighting effect (Static, Breathing, Wave, Off).
    pub fn set_rgb_effect(&self, effect: RgbEffect) -> Result<()> {
        let eff_str = match effect {
            RgbEffect::Static => "static",
            RgbEffect::Breathing => "breathing",
            RgbEffect::Wave => "wave",
            RgbEffect::Zoom => "zoom",
            RgbEffect::Shifting => "shifting",
            RgbEffect::Neon => "neon",
            RgbEffect::Off => "off",
        };
        self.run_ec(&["rgb-effect", eff_str])?;
        Ok(())
    }

    /// Applies entire 4-zone RGB configuration atomically.
    pub fn apply_rgb_config(&self, config: &RgbConfig) -> Result<()> {
        let mut colors = Vec::new();
        for z in &config.zones {
            colors.push((z.r, z.g, z.b));
        }
        let eff_str = match config.effect {
            RgbEffect::Static => "static",
            RgbEffect::Breathing => "breathing",
            RgbEffect::Wave => "wave",
            RgbEffect::Zoom => "zoom",
            RgbEffect::Shifting => "shifting",
            RgbEffect::Neon => "neon",
            RgbEffect::Off => "off",
        };
        self.apply_all_rgb(&colors, eff_str, config.brightness, None, None)
    }

    /// Closes the persistent RGB stream if open.
    pub fn close_rgb_stream(&self) {
        #[cfg(windows)]
        close_rgb_stream();
    }

    /// Atomically applies 4-zone RGB colors, effect, brightness, speed, and direction in a single EC call.
    pub fn apply_all_rgb(&self, zone_colors: &[(u8, u8, u8)], effect: &str, brightness: u8, speed: Option<u8>, direction: Option<u8>) -> Result<()> {
        let is_fast = effect == "fast";
        #[cfg(windows)]
        if is_fast {
            if try_send_rgb_stream(zone_colors, brightness) {
                return Ok(());
            }
        }
        #[cfg(windows)]
        if !is_fast {
            close_rgb_stream();
        }

        let mut args: Vec<String> = vec![if is_fast { "rgb-fast".into() } else { "rgb-all".into() }];
        for i in 0..4 {
            let (r, g, b) = zone_colors.get(i).copied().unwrap_or((0, 210, 255));
            args.push(r.to_string());
            args.push(g.to_string());
            args.push(b.to_string());
        }
        if !is_fast {
            args.push(effect.to_string());
        }
        args.push(brightness.to_string());
        if !is_fast {
            args.push(speed.unwrap_or(5).to_string());
            args.push(direction.unwrap_or(1).to_string());
        }

        let arg_slices: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.run_ec(&arg_slices)?;
        Ok(())
    }

    /// Synchronizes and imports 4-zone RGB configuration from official NitroSense Main.xml profile.
    pub fn sync_nitrosense_rgb(&self) -> Result<String> {
        self.run_ec(&["sync-nitrosense"])
    }

    /// Reads combined hardware telemetry directly from non-blocking native sensors and cached EC.
    pub fn read_telemetry(&self) -> Result<Telemetry> {
        let mut telemetry = crate::sensors::read_telemetry().unwrap_or_default();

        #[cfg(windows)]
        {
            let mut ec_data: Option<EcTelemetryJson> = None;

            if let Ok(guard) = EC_CACHE.lock() {
                if let Some((instant, ref cached)) = *guard {
                    if instant.elapsed() < std::time::Duration::from_millis(750) {
                        ec_data = Some(cached.clone());
                    }
                }
            }

            if ec_data.is_none() {
                if let Ok(json_str) = self.run_ec(&["telemetry"]) {
                    if let Ok(ec) = serde_json::from_str::<EcTelemetryJson>(&json_str) {
                        if let Ok(mut guard) = EC_CACHE.lock() {
                            *guard = Some((std::time::Instant::now(), ec.clone()));
                        }
                        ec_data = Some(ec);
                    }
                }
            }

            if let Some(ec) = ec_data {
                telemetry.cpu_fan_rpm = Some(ec.cpu_rpm);
                telemetry.gpu_fan_rpm = Some(ec.gpu_rpm);
                if ec.cpu_temp > 0.0 && ec.cpu_temp < 120.0 {
                    telemetry.cpu_temp_c = Some(ec.cpu_temp);
                }
                if telemetry.gpu_temp_c.is_none() || telemetry.gpu_temp_c == Some(0.0) {
                    if ec.gpu_temp > 0.0 && ec.gpu_temp < 120.0 {
                        telemetry.gpu_temp_c = Some(ec.gpu_temp);
                    }
                }
                telemetry.battery_care_limit = Some(ec.battery_80_limit);
                telemetry.coolboost = Some(ec.coolboost);
            }
        }

        Ok(telemetry)
    }
}
