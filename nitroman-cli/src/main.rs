use std::env;
use nitroman_core::{AcerDriver, FanMode, RgbEffect, ThermalProfile, UserConfig};

fn print_help() {
    println!("NitroMan Hardware Control CLI");
    println!("Target: Acer Nitro AN515-57 (Windows 11, BIOS 1.17)");
    println!();
    println!("Usage: nitroman-cli <command> [arguments]");
    println!();
    println!("Commands:");
    println!("  status                        Read realtime sensor telemetry (temps, RPMs, battery)");
    println!("  profile <quiet|balanced|perf> Set thermal profile");
    println!("  fan auto                      Set fan mode to automatic curve");
    println!("  fan max                       Set fans to 100% full speed");
    println!("  fan custom <cpu_pct> <gpu_pct> Set manual fan speed percentages (0-100)");
    println!("  battery <on|off>              Toggle 80% battery health conservation limiter");
    println!("  rgb <hex1> <hex2> <hex3> <hex4> Set 4-zone keyboard backlight colors");
    println!("  rgb-effect <static|breathing|wave|off> Set keyboard lighting effect");
    println!("  clean                         Perform temporary disk cleanup");
    println!("  timeout <on|off|secs|get>     Configure keyboard backlight auto-off timeout");
    println!("  config                        Display active saved configuration");
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim_start_matches('#');
    if clean.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
    Some((r, g, b))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    let driver = AcerDriver::new();
    let mut config = UserConfig::load();

    if !driver.is_elevated && args[1] != "config" {
        eprintln!("Notice: Running without elevated Administrator rights. If commands fail to alter physical hardware, run the terminal as Administrator or start via task NitroMan.");
    }

    match args[1].as_str() {
        "status" => {
            println!("========================================================");
            println!("NITROMAN SENSOR TELEMETRY");
            println!("========================================================");
            match driver.read_telemetry() {
                Ok(t) => {
                    let cpu_temp = t.cpu_temp_c.map(|v| format!("{v:.1} C")).unwrap_or_else(|| "N/A".into());
                    let gpu_temp = t.gpu_temp_c.map(|v| format!("{v:.1} C")).unwrap_or_else(|| "N/A".into());
                    let cpu_rpm = t.cpu_fan_rpm.map(|v| format!("{v} RPM")).unwrap_or_else(|| "N/A".into());
                    let gpu_rpm = t.gpu_fan_rpm.map(|v| format!("{v} RPM")).unwrap_or_else(|| "N/A".into());
                    let bat_pct = t.battery_percent.map(|v| format!("{v} %")).unwrap_or_else(|| "N/A".into());

                    println!("  CPU Temperature  : {cpu_temp}");
                    println!("  GPU Temperature  : {gpu_temp}");
                    println!("  CPU Fan Speed    : {cpu_rpm}");
                    println!("  GPU Fan Speed    : {gpu_rpm}");
                    if let Some(load) = t.cpu_load_pct {
                        println!("  CPU Load         : {load:.1} %");
                    }
                    if let Some(mhz) = t.cpu_clock_mhz {
                        println!("  CPU Clock        : {mhz} MHz");
                    }
                    if let Some(pwr) = t.cpu_power_w {
                        println!("  CPU Power        : {pwr:.1} W");
                    }
                    if let Some(g_load) = t.gpu_load_pct {
                        println!("  GPU Load         : {g_load:.1} %");
                    }
                    if let Some(g_mhz) = t.gpu_clock_mhz {
                        println!("  GPU Clock        : {g_mhz} MHz");
                    }
                    if let (Some(used), Some(total)) = (t.vram_used_mb, t.vram_total_mb) {
                        println!("  VRAM Usage       : {used} / {total} MB");
                    }
                    if let (Some(used), Some(total), Some(load)) = (t.ram_used_mb, t.ram_total_gb, t.ram_load_pct) {
                        println!("  RAM Usage        : {used} MB / {total:.1} GB ({load} %)");
                    }
                    if !t.storage_temps.is_empty() {
                        let st_str = t.storage_temps.iter().map(|v| format!("{v} C")).collect::<Vec<_>>().join(" | ");
                        println!("  Storage Drives   : {st_str}");
                    }
                    println!("  Battery Charge   : {bat_pct}");
                    let bat_limit = t.battery_care_limit.unwrap_or(config.battery_80_limit);
                    println!("  Battery 80% Cap  : {}", if bat_limit { "Enabled" } else { "Disabled" });
                    println!("  Thermal Profile  : {:?}", config.thermal_profile);
                    println!("  Fan Mode         : {:?}", config.fan_mode);
                    println!("  Elevated Rights  : {}", if driver.is_elevated { "Yes (Administrator)" } else { "No (Standard User)" });
                }
                Err(e) => {
                    eprintln!("Error reading telemetry: {e}");
                }
            }
            println!("========================================================");
        }

        "profile" => {
            if args.len() < 3 {
                eprintln!("Error: Missing profile argument (quiet, balanced, perf)");
                return;
            }
            let prof = match args[2].to_lowercase().as_str() {
                "quiet" => ThermalProfile::Quiet,
                "balanced" => ThermalProfile::Balanced,
                "perf" | "performance" => ThermalProfile::Performance,
                other => {
                    eprintln!("Unknown profile: {other}. Use quiet, balanced, or perf.");
                    return;
                }
            };

            match driver.set_thermal_profile(prof) {
                Ok(_) => {
                    config.thermal_profile = prof;
                    let _ = config.save();
                    println!("Thermal profile successfully switched to {:?}", prof);
                }
                Err(e) => {
                    eprintln!("Failed to set thermal profile: {e}");
                }
            }
        }

        "fan" => {
            if args.len() < 3 {
                eprintln!("Error: Missing fan mode (auto, max, custom)");
                return;
            }
            match args[2].to_lowercase().as_str() {
                "auto" => {
                    match driver.set_fan_mode(FanMode::Auto) {
                        Ok(_) => {
                            config.fan_mode = FanMode::Auto;
                            let _ = config.save();
                            println!("Fan mode set to Auto (factory EC curve)");
                        }
                        Err(e) => eprintln!("Failed to set fan mode: {e}"),
                    }
                }
                "max" => {
                    match driver.set_fan_mode(FanMode::Performance) {
                        Ok(_) => {
                            config.fan_mode = FanMode::Performance;
                            let _ = config.save();
                            println!("Fan mode set to Maximum (100% full speed)");
                        }
                        Err(e) => eprintln!("Failed to set fan mode: {e}"),
                    }
                }
                "custom" => {
                    if args.len() < 5 {
                        eprintln!("Usage: nitroman-cli fan custom <cpu_pct 0-100> <gpu_pct 0-100>");
                        return;
                    }
                    let cpu: u8 = args[3].parse().unwrap_or(50);
                    let gpu: u8 = args[4].parse().unwrap_or(50);
                    let _ = driver.set_fan_mode(FanMode::Custom);
                    match driver.set_fan_speed(cpu, gpu) {
                        Ok(_) => {
                            config.fan_mode = FanMode::Custom;
                            config.cpu_fan_manual = cpu;
                            config.gpu_fan_manual = gpu;
                            let _ = config.save();
                            println!("Manual fan speeds applied: CPU {cpu}%, GPU {gpu}%");
                        }
                        Err(e) => eprintln!("Failed to set manual fan speed: {e}"),
                    }
                }
                other => eprintln!("Unknown fan command: {other}"),
            }
        }

        "battery" => {
            if args.len() < 3 {
                eprintln!("Usage: nitroman-cli battery <on|off>");
                return;
            }
            let enable = match args[2].to_lowercase().as_str() {
                "on" | "1" | "enable" => true,
                "off" | "0" | "disable" => false,
                _ => {
                    eprintln!("Invalid argument. Use 'on' or 'off'.");
                    return;
                }
            };

            match driver.set_battery_limit_80(enable) {
                Ok(_) => {
                    config.battery_80_limit = enable;
                    let _ = config.save();
                    println!(
                        "Battery 80% charge limiter: {}",
                        if enable { "Enabled (Conservation Mode active)" } else { "Disabled (Charges to 100%)" }
                    );
                }
                Err(e) => eprintln!("Failed to toggle battery limiter: {e}"),
            }
        }

        "rgb" => {
            if args.len() < 6 {
                eprintln!("Usage: nitroman-cli rgb <hex1> <hex2> <hex3> <hex4>");
                eprintln!("Example: nitroman-cli rgb #E4002B #FF4400 #00A3FF #9900FF");
                return;
            }
            for i in 1..=4 {
                if let Some((r, g, b)) = parse_hex_color(&args[i + 1]) {
                    if let Err(e) = driver.set_rgb_zone_color(i as u8, r, g, b) {
                        eprintln!("Warning: Failed to apply Zone {i} to hardware: {e}");
                    }
                    config.rgb.zones[i - 1].r = r;
                    config.rgb.zones[i - 1].g = g;
                    config.rgb.zones[i - 1].b = b;
                } else {
                    eprintln!("Invalid hex color for Zone {i}: {}", args[i + 1]);
                    return;
                }
            }
            let _ = config.save();
            println!("4-zone RGB keyboard colors successfully updated");
        }

        "rgb-effect" => {
            if args.len() < 3 {
                eprintln!("Usage: nitroman-cli rgb-effect <static|breathing|wave|off>");
                return;
            }
            let eff = match args[2].to_lowercase().as_str() {
                "static" => RgbEffect::Static,
                "breathing" => RgbEffect::Breathing,
                "wave" => RgbEffect::Wave,
                "off" => RgbEffect::Off,
                other => {
                    eprintln!("Unknown effect: {other}");
                    return;
                }
            };
            match driver.set_rgb_effect(eff) {
                Ok(_) => {
                    config.rgb.effect = eff;
                    let _ = config.save();
                    println!("Keyboard lighting effect set to {:?}", eff);
                }
                Err(e) => eprintln!("Failed to set lighting effect: {e}"),
            }
        }

        "clean" => {
            println!("Executing temporary Windows disk cleanup...");
            #[cfg(windows)]
            {
                use std::process::Command;
                let _ = Command::new("cmd")
                    .args(["/c", "del /f /q %TEMP%\\* >nul 2>&1"])
                    .output();
            }
            println!("Temporary cache cleaned successfully.");
        }

        "timeout" => {
            if args.len() < 3 {
                match driver.get_backlight_timeout() {
                    Ok(t) => println!("Keyboard backlight timeout: {t}s (0 = always-on)"),
                    Err(e) => eprintln!("Failed to query timeout: {e}"),
                }
                return;
            }
            if args[2].to_lowercase() == "get" {
                match driver.get_backlight_timeout() {
                    Ok(t) => println!("{t}"),
                    Err(e) => eprintln!("Failed to query timeout: {e}"),
                }
                return;
            }
            let secs: u32 = match args[2].to_lowercase().as_str() {
                "off" | "0" | "disable" | "disabled" | "false" => 0,
                "on" | "enable" | "enabled" | "true" => 30,
                num => match num.parse::<u32>() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Invalid timeout value. Use 'on', 'off', or seconds (e.g. 30).");
                        return;
                    }
                },
            };
            match driver.set_backlight_timeout(secs) {
                Ok(_) => {
                    config.backlight_timeout_secs = secs;
                    let _ = config.save();
                    println!(
                        "Keyboard backlight timeout set to {} ({}s)",
                        if secs == 0 { "Always On (no timeout)" } else { "Timeout enabled" },
                        secs
                    );
                }
                Err(e) => eprintln!("Failed to set backlight timeout: {e}"),
            }
        }

        "config" => {
            let json = serde_json::to_string_pretty(&config).unwrap_or_default();
            println!("{json}");
        }

        other => {
            eprintln!("Unknown command: '{other}'");
            print_help();
        }
    }
}
