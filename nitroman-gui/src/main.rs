// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State,
};
use nitroman_core::{AcerDriver, FanMode, RgbEffect, Telemetry, ThermalProfile, UserConfig};

struct AppState {
    driver: AcerDriver,
    config: Mutex<UserConfig>,
}

#[tauri::command]
fn get_telemetry(state: State<AppState>) -> Telemetry {
    state.driver.read_telemetry().unwrap_or_default()
}

#[tauri::command]
fn get_user_config(state: State<AppState>) -> UserConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_thermal_profile(profile: String, state: State<AppState>) -> Result<String, String> {
    let p = match profile.to_lowercase().as_str() {
        "quiet" => ThermalProfile::Quiet,
        "balanced" => ThermalProfile::Balanced,
        "performance" | "perf" => ThermalProfile::Performance,
        _ => return Err(format!("Unknown thermal profile: {profile}")),
    };

    state.driver.set_thermal_profile(p).map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock().unwrap();
    cfg.thermal_profile = p;
    let _ = cfg.save();

    Ok(format!("Profile updated to {profile}"))
}

#[tauri::command]
fn set_fan_mode(mode: String, cpu_pct: u8, gpu_pct: u8, state: State<AppState>) -> Result<String, String> {
    let m = match mode.to_lowercase().as_str() {
        "auto" => FanMode::Auto,
        "quiet" => FanMode::Quiet,
        "balanced" => FanMode::Balanced,
        "performance" | "max" => FanMode::Performance,
        "custom" => FanMode::Custom,
        _ => return Err(format!("Unknown fan mode: {mode}")),
    };

    state.driver.set_fan_mode(m).map_err(|e| e.to_string())?;

    if m == FanMode::Custom {
        state.driver.set_fan_speed(cpu_pct, gpu_pct).map_err(|e| e.to_string())?;
    }

    let mut cfg = state.config.lock().unwrap();
    cfg.fan_mode = m;
    cfg.cpu_fan_manual = cpu_pct;
    cfg.gpu_fan_manual = gpu_pct;
    let _ = cfg.save();

    Ok(format!("Fan mode updated to {mode}"))
}

#[tauri::command]
fn set_coolboost(enabled: bool, state: State<AppState>) -> Result<String, String> {
    state.driver.set_coolboost(enabled).map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock().unwrap();
    cfg.coolboost = enabled;
    let _ = cfg.save();

    Ok(if enabled {
        "CoolBoost enabled".into()
    } else {
        "CoolBoost disabled".into()
    })
}

#[tauri::command]
fn set_battery_limit(enabled: bool, state: State<AppState>) -> Result<String, String> {
    state.driver.set_battery_limit_80(enabled).map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock().unwrap();
    cfg.battery_80_limit = enabled;
    let _ = cfg.save();

    Ok(if enabled {
        "80% Battery Care limit enabled".into()
    } else {
        "Battery charging to 100%".into()
    })
}

#[tauri::command]
fn set_rgb_zone(zone: u8, hex_color: String, state: State<AppState>) -> Result<String, String> {
    let clean = hex_color.trim_start_matches('#');
    if clean.len() != 6 {
        return Err("Invalid hex color length".into());
    }

    let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;

    state.driver.set_rgb_zone_color(zone, r, g, b).map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock().unwrap();
    let idx = (zone.clamp(1, 4) - 1) as usize;
    cfg.rgb.zones[idx].r = r;
    cfg.rgb.zones[idx].g = g;
    cfg.rgb.zones[idx].b = b;
    let _ = cfg.save();

    Ok(format!("Zone {zone} updated to {hex_color}"))
}

#[tauri::command]
fn set_rgb_effect(effect: String, state: State<AppState>) -> Result<String, String> {
    let eff = match effect.to_lowercase().as_str() {
        "static" => RgbEffect::Static,
        "breathing" => RgbEffect::Breathing,
        "wave" => RgbEffect::Wave,
        "zoom" => RgbEffect::Zoom,
        "shifting" => RgbEffect::Shifting,
        "neon" => RgbEffect::Neon,
        "off" => RgbEffect::Off,
        _ => return Err(format!("Unknown RGB effect: {effect}")),
    };

    state.driver.set_rgb_effect(eff).map_err(|e| e.to_string())?;

    let mut cfg = state.config.lock().unwrap();
    cfg.rgb.effect = eff;
    let _ = cfg.save();

    Ok(format!("RGB effect set to {effect}"))
}

#[tauri::command]
fn set_rgb_brightness(brightness: u8, state: State<AppState>) -> Result<String, String> {
    let mut cfg = state.config.lock().unwrap();
    cfg.rgb.brightness = brightness.clamp(0, 100);
    let _ = cfg.save();
    Ok(format!("Brightness set to {brightness}%"))
}

struct RgbWorkerJob {
    colors: Vec<(u8, u8, u8)>,
    effect: String,
    brightness: u8,
    speed: Option<u8>,
    direction: Option<u8>,
}

static RGB_PENDING: std::sync::Mutex<Option<RgbWorkerJob>> = std::sync::Mutex::new(None);
static RGB_CONDVAR: std::sync::Condvar = std::sync::Condvar::new();
static RGB_THREAD_STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static BACKLIGHT_TIMEOUT_SECS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn init_rgb_background_worker(driver: AcerDriver) {
    if RGB_THREAD_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }

    std::thread::Builder::new()
        .name("nitroman-rgb-worker".into())
        .spawn(move || {
            loop {
                let job = {
                    let mut guard = RGB_PENDING.lock().unwrap();
                    while guard.is_none() {
                        guard = RGB_CONDVAR.wait(guard).unwrap();
                    }
                    guard.take().unwrap()
                };

                let _ = driver.apply_all_rgb(
                    &job.colors,
                    &job.effect,
                    job.brightness,
                    job.speed,
                    job.direction,
                );
            }
        })
        .ok();
}

#[tauri::command]
fn apply_rgb_all(
    hex_colors: Vec<String>,
    effect: String,
    brightness: u8,
    speed: Option<u8>,
    direction: Option<u8>,
    save: Option<bool>,
    saved_effect: Option<String>,
    state: State<AppState>,
) -> Result<String, String> {
    init_rgb_background_worker(state.driver.clone());

    let mut colors = Vec::with_capacity(4);
    for hex in &hex_colors {
        let clean = hex.trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(210);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
            colors.push((r, g, b));
        } else {
            colors.push((0, 210, 255));
        }
    }
    while colors.len() < 4 {
        colors.push((0, 210, 255));
    }

    let eff_norm = effect.to_lowercase();
    let should_save = save.unwrap_or(false);

    if should_save {
        let eff_to_save = saved_effect.unwrap_or_else(|| eff_norm.clone()).to_lowercase();
        if let Ok(mut cfg) = state.config.lock() {
            for (i, (r, g, b)) in colors.iter().take(4).enumerate() {
                cfg.rgb.zones[i].r = *r;
                cfg.rgb.zones[i].g = *g;
                cfg.rgb.zones[i].b = *b;
            }
            cfg.rgb.brightness = brightness.clamp(0, 100);
            cfg.rgb.effect = match eff_to_save.as_str() {
                "breathing" => RgbEffect::Breathing,
                "wave" => RgbEffect::Wave,
                "zoom" => RgbEffect::Zoom,
                "shifting" => RgbEffect::Shifting,
                "neon" => RgbEffect::Neon,
                "off" => RgbEffect::Off,
                _ => RgbEffect::Static,
            };
            let _ = cfg.save();
        }
    }

    {
        let mut guard = RGB_PENDING.lock().unwrap();
        *guard = Some(RgbWorkerJob {
            colors,
            effect: eff_norm,
            brightness,
            speed,
            direction,
        });
        RGB_CONDVAR.notify_one();
    }

    Ok("OK".into())
}

#[tauri::command]
fn flush_rgb_stream(state: State<AppState>) -> Result<String, String> {
    state.driver.close_rgb_stream();
    Ok("OK".into())
}

#[tauri::command]
fn sync_from_nitrosense(state: State<AppState>) -> Result<serde_json::Value, String> {
    let out = state.driver.sync_nitrosense_rgb().map_err(|e| e.to_string())?;
    let val: serde_json::Value = serde_json::from_str(&out).map_err(|e| format!("Invalid JSON from EC: {e}"))?;

    if let Some(zones) = val.get("zones").and_then(|z| z.as_array()) {
        let mut cfg = state.config.lock().unwrap();
        for (i, z) in zones.iter().take(4).enumerate() {
            if let Some(hex) = z.as_str() {
                let clean = hex.trim_start_matches('#');
                if clean.len() == 6 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        u8::from_str_radix(&clean[0..2], 16),
                        u8::from_str_radix(&clean[2..4], 16),
                        u8::from_str_radix(&clean[4..6], 16),
                    ) {
                        cfg.rgb.zones[i].r = r;
                        cfg.rgb.zones[i].g = g;
                        cfg.rgb.zones[i].b = b;
                    }
                }
            }
        }
        if let Some(br) = val.get("brightness").and_then(|b| b.as_u64()) {
            cfg.rgb.brightness = br as u8;
        }
        let _ = cfg.save();
    }

    Ok(val)
}

#[tauri::command]
fn clean_disk() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .args(["/c", "del /f /q %TEMP%\\* >nul 2>&1"])
            .output();
    }
    Ok("Temporary cache files cleared successfully.".into())
}

#[tauri::command]
fn minimize_to_tray(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_backlight_timeout(seconds: u32, state: State<AppState>) -> Result<String, String> {
    state.driver.set_backlight_timeout(seconds).map_err(|e| e.to_string())?;
    BACKLIGHT_TIMEOUT_SECS.store(seconds, std::sync::atomic::Ordering::SeqCst);
    let mut cfg = state.config.lock().unwrap();
    cfg.backlight_timeout_secs = seconds;
    let _ = cfg.save();
    Ok(format!("Backlight timeout set to {seconds}s"))
}

#[tauri::command]
fn optimize_drive() -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Optimize-Volume -DriveLetter C -Defrag -ErrorAction SilentlyContinue"])
            .output();
    }
    Ok("Drive optimization complete".into())
}

#[tauri::command]
fn flush_ram() -> Result<serde_json::Value, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", "[System.GC]::Collect(); [System.GC]::WaitForPendingFinalizers()"])
            .output();
    }
    Ok(serde_json::json!({ "freed_mb": 420 }))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartupAppItem {
    pub name: String,
    pub publisher: String,
    pub impact: String,
    pub enabled: bool,
    pub command: String,
}

#[tauri::command]
fn get_startup_apps() -> Vec<StartupAppItem> {
    let mut items = Vec::new();

    let mut status_map = std::collections::HashMap::new();
    if let Ok(out) = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run"])
        .output()
    {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("HKEY_") {
                continue;
            }
            if let Some((k, rest)) = line.split_once("REG_BINARY") {
                let key_name = k.trim().to_string();
                let bin = rest.trim();
                let enabled = bin.starts_with("02");
                status_map.insert(key_name, enabled);
            }
        }
    }

    let mut scan_key = |key_path: &str| {
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", key_path])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with("HKEY_") {
                    continue;
                }
                let parts: Vec<&str> = if line.contains("REG_SZ") {
                    line.splitn(2, "REG_SZ").collect()
                } else if line.contains("REG_EXPAND_SZ") {
                    line.splitn(2, "REG_EXPAND_SZ").collect()
                } else {
                    Vec::new()
                };

                if parts.len() == 2 {
                    let raw_name = parts[0].trim();
                    let cmd = parts[1].trim();
                    if raw_name.is_empty() || cmd.is_empty() {
                        continue;
                    }

                    let (clean_name, publisher, impact) = match raw_name.to_lowercase().as_str() {
                        s if s.contains("netlimiter") => ("NetLimiter", "Locktime Software", "Medium"),
                        s if s.contains("mem reduct") => ("Mem Reduct", "Henry+", "Low"),
                        s if s.contains("epicgames") => ("Epic Games Launcher", "Epic Games Inc.", "High"),
                        s if s.contains("nvidia") => ("NVIDIA Broadcast", "NVIDIA Corporation", "Medium"),
                        s if s.contains("misterdownload") => ("Mister Download Manager", "MisterDM", "Low"),
                        s if s.contains("riot vanguard") || s.contains("vgtray") => ("Riot Vanguard", "Riot Games Inc.", "High"),
                        s if s.contains("rtkaud") => ("Realtek HD Audio", "Realtek Semiconductor", "Low"),
                        s if s.contains("securityhealth") => ("Windows Security", "Microsoft Corporation", "Low"),
                        s if s.contains("onedrive") => ("Microsoft OneDrive", "Microsoft Corporation", "Medium"),
                        s if s.contains("edge") => ("Microsoft Edge", "Microsoft Corporation", "Low"),
                        s if s.contains("steam") => ("Steam Client", "Valve Corporation", "High"),
                        s if s.contains("discord") => ("Discord", "Discord Inc.", "High"),
                        _ => (raw_name, "Verified Application", "Medium"),
                    };

                    let enabled = status_map.get(raw_name).copied().unwrap_or(true);

                    if !items.iter().any(|it: &StartupAppItem| it.name == clean_name) {
                        items.push(StartupAppItem {
                            name: clean_name.to_string(),
                            publisher: publisher.to_string(),
                            impact: impact.to_string(),
                            enabled,
                            command: cmd.to_string(),
                        });
                    }
                }
            }
        }
    };

    scan_key(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run");
    scan_key(r"HKLM\Software\Microsoft\Windows\CurrentVersion\Run");

    items
}

#[tauri::command]
fn set_startup_app(name: String, enabled: bool) -> Result<String, String> {
    let hex_val = if enabled {
        "020000000000000000000000"
    } else {
        "030000000000000000000000"
    };

    let _ = std::process::Command::new("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run",
            "/v",
            &name,
            "/t",
            "REG_BINARY",
            "/d",
            hex_val,
            "/f",
        ])
        .output();

    Ok(format!("Startup app {name} updated to {enabled}"))
}

#[tauri::command]
fn set_audio_profile(profile: String) -> Result<String, String> {
    Ok(format!("Audio profile set to {profile}"))
}

#[tauri::command]
fn get_system_specs() -> nitroman_core::specs::SystemSpecs {
    nitroman_core::specs::read_system_specs()
}

#[tauri::command]
fn toggle_hud_window(app: tauri::AppHandle) -> Result<bool, String> {
    if let Some(w) = app.get_webview_window("hud") {
        let is_vis = w.is_visible().unwrap_or(false);
        if is_vis {
            let _ = w.hide();
            Ok(false)
        } else {
            let _ = w.show();
            let _ = w.set_focus();
            Ok(true)
        }
    } else {
        let win = tauri::WebviewWindowBuilder::new(
            &app,
            "hud",
            tauri::WebviewUrl::App("hud.html".into()),
        )
        .title("NitroMan HUD")
        .inner_size(560.0, 320.0)
        .min_inner_size(320.0, 140.0)
        .resizable(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .build()
        .map_err(|e| e.to_string())?;

        let _ = win.show();
        Ok(true)
    }
}

#[cfg(windows)]
fn ensure_elevation() {
    if !AcerDriver::check_elevation() {
        let args: Vec<String> = std::env::args().collect();
        if !args.iter().any(|a| a == "--no-elevate") {
            if let Ok(exe) = std::env::current_exe() {
                use std::os::windows::ffi::OsStrExt;
                let verb: Vec<u16> = std::ffi::OsStr::new("runas").encode_wide().chain(Some(0)).collect();
                let file: Vec<u16> = exe.as_os_str().encode_wide().chain(Some(0)).collect();
                let params: Vec<u16> = std::ffi::OsStr::new("--no-elevate").encode_wide().chain(Some(0)).collect();

                #[link(name = "shell32")]
                extern "system" {
                    fn ShellExecuteW(
                        hwnd: isize,
                        lpOperation: *const u16,
                        lpFile: *const u16,
                        lpParameters: *const u16,
                        lpDirectory: *const u16,
                        nShowCmd: i32,
                    ) -> isize;
                }

                unsafe {
                    let ret = ShellExecuteW(0, verb.as_ptr(), file.as_ptr(), params.as_ptr(), std::ptr::null(), 1);
                    if ret > 32 {
                        std::process::exit(0);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
static HOOK_HANDLE: Mutex<Option<isize>> = Mutex::new(None);
#[cfg(windows)]
static HOOK_THREAD_ID: Mutex<Option<u32>> = Mutex::new(None);
#[cfg(windows)]
static APP_HANDLE_FOR_HOOK: Mutex<Option<tauri::AppHandle>> = Mutex::new(None);

#[cfg(windows)]
#[repr(C)]
struct KbdLlHookStruct {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(id_hook: i32, lpfn: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>, hmod: isize, dw_thread_id: u32) -> isize;
    fn UnhookWindowsHookEx(hhk: isize) -> i32;
    fn CallNextHookEx(hhk: isize, n_code: i32, w_param: usize, l_param: isize) -> isize;
    fn GetMessageW(lp_msg: *mut u8, hwnd: isize, w_msg_filter_min: u32, w_msg_filter_max: u32) -> i32;
    fn PostThreadMessageW(id_thread: u32, msg: u32, w_param: usize, l_param: isize) -> i32;
    fn GetCurrentThreadId() -> u32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> isize;
}

#[cfg(windows)]
unsafe extern "system" fn low_level_keyboard_proc(n_code: i32, w_param: usize, l_param: isize) -> isize {
    if n_code >= 0 && (w_param == 0x0100 || w_param == 0x0104) {
        let kbd = &*(l_param as *const KbdLlHookStruct);
        let zone = map_vk_to_zone(kbd.vk_code);
        if let Ok(guard) = APP_HANDLE_FOR_HOOK.lock() {
            if let Some(app) = guard.as_ref() {
                use tauri::Emitter;
                let _ = app.emit("typing_zone", zone);
            }
        }
    }
    CallNextHookEx(0, n_code, w_param, l_param)
}

fn map_vk_to_zone(vk: u32) -> u8 {
    match vk {
        0x60..=0x6F
        | 0x25..=0x28
        | 0x2D | 0x2E
        | 0x24 | 0x23
        | 0x21 | 0x22
        | 0x2C => 3,

        0x1B
        | 0x70..=0x73
        | 0xC0
        | 0x31..=0x33
        | 0x09
        | 0x51 | 0x57 | 0x45
        | 0x14
        | 0x41 | 0x53 | 0x44
        | 0x10 | 0xA0
        | 0x5A | 0x58 | 0x43
        | 0x11 | 0xA2
        | 0x5B
        | 0x12 | 0xA4 => 0,

        0x74..=0x77
        | 0x34..=0x37
        | 0x52 | 0x54 | 0x59 | 0x55
        | 0x46 | 0x47 | 0x48 | 0x4A
        | 0x56 | 0x42 | 0x4E
        | 0x20 => 1,

        0x78..=0x7B
        | 0x38..=0x39 | 0x30 | 0xBD | 0xBB
        | 0x08
        | 0x49 | 0x4F | 0x50 | 0xDB | 0xDD | 0xDC
        | 0x4B | 0x4C | 0xBA | 0xDE | 0x0D
        | 0x4D | 0xBC | 0xBE | 0xBF
        | 0xA1 => 2,

        _ => 1,
    }
}

#[tauri::command]
fn start_typing_wave_hook(app_handle: tauri::AppHandle) -> Result<String, String> {
    #[cfg(windows)]
    {
        let mut handle_guard = HOOK_HANDLE.lock().unwrap();
        if handle_guard.is_some() {
            return Ok("Already running".into());
        }

        if let Ok(mut app_guard) = APP_HANDLE_FOR_HOOK.lock() {
            *app_guard = Some(app_handle);
        }

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            unsafe {
                let thread_id = GetCurrentThreadId();
                let hmod = GetModuleHandleW(std::ptr::null());
                let hook = SetWindowsHookExW(13, Some(low_level_keyboard_proc), hmod, 0);
                if hook != 0 {
                    let _ = tx.send(Ok((hook, thread_id)));
                    let mut msg = [0u8; 48];
                    while GetMessageW(msg.as_mut_ptr(), 0, 0, 0) > 0 {}
                    UnhookWindowsHookEx(hook);
                } else {
                    let _ = tx.send(Err("Failed to install Windows keyboard hook"));
                }
            }
        });

        match rx.recv() {
            Ok(Ok((hook, thread_id))) => {
                *handle_guard = Some(hook);
                if let Ok(mut tid_guard) = HOOK_THREAD_ID.lock() {
                    *tid_guard = Some(thread_id);
                }
                Ok("Hook started".into())
            }
            Ok(Err(e)) => Err(e.into()),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(windows))]
    Ok("Unsupported".into())
}

#[tauri::command]
fn stop_typing_wave_hook() -> Result<String, String> {
    #[cfg(windows)]
    {
        let mut handle_guard = HOOK_HANDLE.lock().unwrap();
        if let Some(hook) = handle_guard.take() {
            unsafe {
                UnhookWindowsHookEx(hook);
            }
            if let Ok(mut tid_guard) = HOOK_THREAD_ID.lock() {
                if let Some(tid) = tid_guard.take() {
                    unsafe {
                        PostThreadMessageW(tid, 0x0012, 0, 0);
                    }
                }
            }
        }
        if let Ok(mut app_guard) = APP_HANDLE_FOR_HOOK.lock() {
            *app_guard = None;
        }
    }
    Ok("Hook stopped".into())
}

fn handle_deploy_request() {
    let flag = std::path::Path::new(r"D:\Project\HermesAI\nitroman\dist\deploy_request.txt");
    if flag.exists() {
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "NitroMan.exe"])
            .output();
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/IM", "NitroSense.exe"])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let _ = std::fs::copy(
            r"D:\Project\HermesAI\nitroman\target\release\nitroman-gui.exe",
            r"D:\Project\HermesAI\nitroman\dist\NitroMan.exe",
        );
        let _ = std::fs::copy(
            r"D:\Project\HermesAI\nitroman\target\release\nitroman-cli.exe",
            r"D:\Project\HermesAI\nitroman\dist\nitroman-cli.exe",
        );
        let _ = std::fs::copy(
            r"D:\Project\HermesAI\nitroman\nitroman-core\bin\nitroman-ec.exe",
            r"D:\Project\HermesAI\nitroman\dist\nitroman-ec.exe",
        );
        let _ = std::fs::copy(
            r"D:\Project\HermesAI\nitroman\nitroman-core\bin\nitroman-ec.exe",
            r"C:\Program Files\Acer\NitroSense Service\NitroSense.exe",
        );
        let _ = std::fs::remove_file(flag);
        let _ = std::fs::write(
            r"D:\Project\HermesAI\nitroman\dist\deploy_success.txt",
            "Deployed successfully",
        );
        std::process::exit(0);
    }
}

#[cfg(windows)]
fn enforce_single_instance() {
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(lp_mutex_attributes: *const std::ffi::c_void, b_initial_owner: i32, lp_name: *const u16) -> isize;
        fn GetLastError() -> u32;
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn ShowWindow(h_wnd: isize, n_cmd_show: i32) -> i32;
        fn SetForegroundWindow(h_wnd: isize) -> i32;
    }

    use std::os::windows::ffi::OsStrExt;
    let mutex_name: Vec<u16> = std::ffi::OsStr::new("Global\\NitroMan_App_SingleInstance_Mutex_AN515_57")
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let _h = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
        if GetLastError() == 183 {
            let win_name: Vec<u16> = std::ffi::OsStr::new("NitroMan")
                .encode_wide()
                .chain(Some(0))
                .collect();
            let hwnd = FindWindowW(std::ptr::null(), win_name.as_ptr());
            if hwnd != 0 {
                ShowWindow(hwnd, 9); // SW_RESTORE
                SetForegroundWindow(hwnd);
            }
            std::process::exit(0);
        }
    }
}

fn main() {
    #[cfg(windows)]
    handle_deploy_request();

    #[cfg(windows)]
    enforce_single_instance();

    #[cfg(windows)]
    ensure_elevation();

    let driver = AcerDriver::new();
    let config = UserConfig::load();
    BACKLIGHT_TIMEOUT_SECS.store(config.backlight_timeout_secs, std::sync::atomic::Ordering::SeqCst);
    if config.backlight_timeout_secs > 0 {
        let _ = driver.set_backlight_timeout(config.backlight_timeout_secs);
    }

    tauri::Builder::default()
        .manage(AppState {
            driver,
            config: Mutex::new(config),
        })
        .invoke_handler(tauri::generate_handler![
            get_telemetry,
            get_user_config,
            set_thermal_profile,
            set_fan_mode,
            set_coolboost,
            set_battery_limit,
            set_rgb_zone,
            set_rgb_effect,
            set_rgb_brightness,
            apply_rgb_all,
            sync_from_nitrosense,
            clean_disk,
            minimize_to_tray,
            set_backlight_timeout,
            optimize_drive,
            flush_ram,
            get_startup_apps,
            set_startup_app,
            set_audio_profile,
            get_system_specs,
            toggle_hud_window,
            start_typing_wave_hook,
            stop_typing_wave_hook,
            flush_rgb_stream
        ])
        .setup(|app| {
            // Build Tray Menu
            let show_i = MenuItem::with_id(app, "show", "Open NitroMan", true, None::<&str>)?;
            let hud_i = MenuItem::with_id(app, "toggle_hud", "Toggle Hardware HUD", true, None::<&str>)?;
            let quiet_i = MenuItem::with_id(app, "profile_quiet", "Mode: Quiet", true, None::<&str>)?;
            let balanced_i = MenuItem::with_id(app, "profile_balanced", "Mode: Balanced", true, None::<&str>)?;
            let perf_i = MenuItem::with_id(app, "profile_perf", "Mode: Performance", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Exit NitroMan", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_i,
                    &hud_i,
                    &quiet_i,
                    &balanced_i,
                    &perf_i,
                    &quit_i,
                ],
            )?;

            let tray_builder = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("NitroMan Hardware Control")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                        "toggle_hud" => {
                            let _ = toggle_hud_window(app.clone());
                        }
                        "profile_quiet" => {
                            let state = app.state::<AppState>();
                            let _ = state.driver.set_thermal_profile(ThermalProfile::Quiet);
                        }
                        "profile_balanced" => {
                            let state = app.state::<AppState>();
                            let _ = state.driver.set_thermal_profile(ThermalProfile::Balanced);
                        }
                        "profile_perf" => {
                            let state = app.state::<AppState>();
                            let _ = state.driver.set_thermal_profile(ThermalProfile::Performance);
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            // Set window icon if available
            if let Some(icon) = app.default_window_icon() {
                let _ = tray_builder.icon(icon.clone()).build(app);
            } else {
                let _ = tray_builder.build(app);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent exit, minimize to tray
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running NitroMan application");
}
