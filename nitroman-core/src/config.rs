//! User preferences and persistence layer for NitroMan.
//!
//! Stores fan curves, RGB color configurations, power profile selections,
//! and battery health limit flags to local JSON storage.

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{FanCurvePoint, FanMode, RgbConfig, ThermalProfile};

/// Complete persisted user configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserConfig {
    /// Active thermal profile.
    pub thermal_profile: ThermalProfile,
    /// Active fan mode.
    pub fan_mode: FanMode,
    /// CoolBoost state (forces higher ceiling RPM under thermal stress).
    pub coolboost: bool,
    /// 80 percent charge limit toggle (Battery Health / Conservation Mode).
    pub battery_80_limit: bool,
    /// Manual CPU fan speed percentage (0..=100).
    pub cpu_fan_manual: u8,
    /// Manual GPU fan speed percentage (0..=100).
    pub gpu_fan_manual: u8,
    /// Custom fan curve threshold points.
    pub fan_curve: Vec<FanCurvePoint>,
    /// 4-Zone keyboard RGB configuration.
    pub rgb: RgbConfig,
    /// Telemetry polling interval in milliseconds (e.g. 1000).
    pub polling_interval_ms: u64,
    /// Minimize to system tray on window close.
    pub minimize_to_tray: bool,
    /// Launch at Windows startup via elevated Task Scheduler.
    pub autostart_elevated: bool,
    /// Keyboard backlight auto-off timeout in seconds (0 = disabled/always-on, 30 = 30s).
    #[serde(default)]
    pub backlight_timeout_secs: u32,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            thermal_profile: ThermalProfile::Balanced,
            fan_mode: FanMode::Auto,
            coolboost: true,
            battery_80_limit: true,
            cpu_fan_manual: 50,
            gpu_fan_manual: 50,
            fan_curve: vec![
                FanCurvePoint::new(40, 30),
                FanCurvePoint::new(55, 45),
                FanCurvePoint::new(70, 65),
                FanCurvePoint::new(80, 85),
                FanCurvePoint::new(90, 100),
            ],
            rgb: RgbConfig::default(),
            polling_interval_ms: 1000,
            minimize_to_tray: true,
            autostart_elevated: true,
            backlight_timeout_secs: 0,
        }
    }
}

impl UserConfig {
    /// Resolves standard config path: `%APPDATA%\NitroMan\config.json`.
    pub fn default_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data).join("NitroMan").join("config.json")
        } else {
            PathBuf::from("config.json")
        }
    }

    /// Loads config from disk if exists, otherwise returns default.
    pub fn load() -> Self {
        let path = Self::default_path();
        Self::load_from_path(&path).unwrap_or_default()
    }

    /// Loads config from specific file path.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(Error::Unsupported(format!(
                "Config file does not exist at {}",
                path.display()
            )));
        }
        let data = fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&data)?;
        Ok(config)
    }

    /// Saves config to default storage path, creating parent directories if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        self.save_to_path(&path)
    }

    /// Saves config to specific path.
    pub fn save_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let json_str = serde_json::to_string_pretty(self)?;
        fs::write(path, json_str)?;
        Ok(())
    }

    /// Validates internal value bounds.
    pub fn validate(&mut self) {
        self.cpu_fan_manual = self.cpu_fan_manual.min(100);
        self.gpu_fan_manual = self.gpu_fan_manual.min(100);
        self.rgb.brightness = self.rgb.brightness.min(100);
        if self.polling_interval_ms < 200 {
            self.polling_interval_ms = 200;
        }
        for pt in &mut self.fan_curve {
            pt.temp_c = pt.temp_c.clamp(30, 100);
            pt.fan_percent = pt.fan_percent.min(100);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ZoneColor;

    #[test]
    fn test_default_config_validity() {
        let mut cfg = UserConfig::default();
        cfg.validate();
        assert_eq!(cfg.thermal_profile, ThermalProfile::Balanced);
        assert_eq!(cfg.fan_mode, FanMode::Auto);
        assert!(cfg.coolboost);
        assert!(cfg.battery_80_limit);
        assert_eq!(cfg.cpu_fan_manual, 50);
        assert_eq!(cfg.gpu_fan_manual, 50);
        assert_eq!(cfg.fan_curve.len(), 5);
        assert_eq!(cfg.rgb.brightness, 50);
    }

    #[test]
    fn test_config_json_roundtrip() {
        let mut original = UserConfig::default();
        original.cpu_fan_manual = 75;
        original.battery_80_limit = false;
        original.rgb.zones[0] = ZoneColor::new(crate::model::Zone::Zone1, 0, 255, 128);

        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: UserConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_config_save_load_tempfile() {
        let temp_dir = std::env::temp_dir().join("nitroman_test_config");
        let config_file = temp_dir.join("test_config.json");

        let mut original = UserConfig::default();
        original.coolboost = false;
        original.save_to_path(&config_file).expect("save");

        let loaded = UserConfig::load_from_path(&config_file).expect("load");
        assert_eq!(original, loaded);

        // Cleanup
        let _ = fs::remove_file(&config_file);
        let _ = fs::remove_dir(&temp_dir);
    }
}
