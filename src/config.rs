use crate::l10n::Locale;
use crate::settings::{SpeedUnit, TemperatureUnit, ThemeMode};
use crate::weather::GeoResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SavedConfig {
    #[serde(default)]
    pub settings: SavedSettings,
    #[serde(default)]
    pub carousels: SavedCarousels,
    #[serde(default)]
    pub widgets: Vec<SavedWidgetPrefs>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedSettings {
    pub theme_mode: ThemeMode,
    pub theme_dark_at: String,
    pub theme_light_at: String,
    pub smooth_tick: bool,
    pub locale: Locale,
    pub temperature_unit: TemperatureUnit,
    pub speed_unit: SpeedUnit,
}

impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::default(),
            theme_dark_at: "21:00".into(),
            theme_light_at: "08:00".into(),
            smooth_tick: true,
            locale: Locale::default(),
            temperature_unit: TemperatureUnit::default(),
            speed_unit: SpeedUnit::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SavedCarousels {
    pub page0_left: usize,
    pub page0_right: usize,
    pub page1: usize,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SavedWidgetPrefs {
    pub id: usize,
    pub selected_city: Option<GeoResult>,
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("standbyrsd")
        .join("config.toml")
}

impl SavedConfig {
    pub fn load() -> Self {
        let path = config_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, s);
        }
    }
}
