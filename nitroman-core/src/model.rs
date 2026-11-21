use serde::{Deserialize, Serialize};

/// Fan operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FanMode {
    Auto,
    Quiet,
    Balanced,
    Performance,
    Custom,
}

impl FanMode {
    pub const ALL: &'static [FanMode] = &[
        FanMode::Auto,
        FanMode::Quiet,
        FanMode::Balanced,
        FanMode::Performance,
        FanMode::Custom,
    ];
}

/// System thermal profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalProfile {
    Quiet,
    Balanced,
    Performance,
}

impl ThermalProfile {
    pub const ALL: &'static [ThermalProfile] = &[
        ThermalProfile::Quiet,
        ThermalProfile::Balanced,
        ThermalProfile::Performance,
    ];
}

/// Single point on custom fan curve. `temp_c` in 40..=90.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FanCurvePoint {
    pub temp_c: u8,
    pub fan_percent: u8,
}

impl FanCurvePoint {
    pub fn new(temp_c: u8, fan_percent: u8) -> Self {
        Self {
            temp_c,
            fan_percent,
        }
    }
}

/// RGB zone identifier (AN515-57 has 4 physical zones).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Zone {
    Zone1 = 0,
    Zone2 = 1,
    Zone3 = 2,
    Zone4 = 3,
}

impl Zone {
    pub const ALL: &'static [Zone] = &[Zone::Zone1, Zone::Zone2, Zone::Zone3, Zone::Zone4];

    pub fn index(self) -> usize {
        self as usize
    }
}

/// RGB effect preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RgbEffect {
    Static,
    Breathing,
    Wave,
    Zoom,
    Shifting,
    Neon,
    Off,
}

/// RGB zone color as (r, g, b) 0-255.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneColor {
    pub zone: Zone,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ZoneColor {
    pub fn new(zone: Zone, r: u8, g: u8, b: u8) -> Self {
        Self { zone, r, g, b }
    }
    pub const WHITE: [u8; 3] = [255, 255, 255];
}

/// Complete RGB keyboard configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbConfig {
    pub zones: [ZoneColor; 4],
    pub effect: RgbEffect,
    /// Brightness 0..=100
    pub brightness: u8,
}

impl Default for RgbConfig {
    fn default() -> Self {
        Self {
            zones: [
                ZoneColor::new(Zone::Zone1, 228, 0, 43),
                ZoneColor::new(Zone::Zone2, 228, 0, 43),
                ZoneColor::new(Zone::Zone3, 228, 0, 43),
                ZoneColor::new(Zone::Zone4, 228, 0, 43),
            ],
            effect: RgbEffect::Static,
            brightness: 50,
        }
    }
}

/// Battery configuration. AN515-57 only supports the 80% conservation threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatteryConfig {
    /// When true, charging stops at 80% (Conservation Mode).
    pub limit_80_enabled: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            limit_80_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fan_mode_serde() {
        assert_eq!(
            serde_json::to_string(&FanMode::Balanced).unwrap(),
            "\"balanced\""
        );
    }
    #[test]
    fn rgb_default_red_accent() {
        let c = RgbConfig::default();
        assert_eq!(c.zones[0].r, 228);
    }
}
