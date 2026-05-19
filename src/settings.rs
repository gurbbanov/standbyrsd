use crate::l10n::Locale;
use iced::widget::combo_box;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub theme_mode: ThemeMode,
    pub theme_mode_combo: combo_box::State<ThemeMode>,
    pub theme_dark_at: String,
    pub theme_light_at: String,
    pub smooth_tick: bool,
    pub locale: Locale,
    pub locale_combo: combo_box::State<Locale>,
    pub temperature_unit: TemperatureUnit,
    pub temperature_combo: combo_box::State<TemperatureUnit>,
    pub speed_unit: SpeedUnit,
    pub speed_combo: combo_box::State<SpeedUnit>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Classic,
            theme_mode_combo: combo_box::State::new(vec![
                ThemeMode::Classic,
                ThemeMode::RedDark,
                ThemeMode::AutoSunrise,
                ThemeMode::AutoCustom,
            ]),
            theme_dark_at: "22:00".to_string(),
            theme_light_at: "07:00".to_string(),
            smooth_tick: true,
            locale: Locale::En,
            locale_combo: combo_box::State::new(Locale::all()),
            temperature_unit: TemperatureUnit::default(),
            temperature_combo: combo_box::State::new(vec![
                TemperatureUnit::Celsius,
                TemperatureUnit::Fahrenheit,
            ]),
            speed_unit: SpeedUnit::default(),
            speed_combo: combo_box::State::new(vec![SpeedUnit::KmH, SpeedUnit::Ms, SpeedUnit::Mph]),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ThemeMode {
    #[default]
    Classic,
    RedDark,
    AutoSunrise,
    AutoCustom,
}

impl std::fmt::Display for ThemeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeMode::Classic => write!(f, "classic"),
            ThemeMode::RedDark => write!(f, "red dark"),
            ThemeMode::AutoSunrise => write!(f, "auto (sunrise/sunset)"),
            ThemeMode::AutoCustom => write!(f, "auto (custom hours)"),
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

impl std::fmt::Display for TemperatureUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemperatureUnit::Celsius => write!(f, "celsius (°C)"),
            TemperatureUnit::Fahrenheit => write!(f, "fahrenheit (°F)"),
        }
    }
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub enum SpeedUnit {
    #[default]
    KmH,
    Ms,
    Mph,
}

impl std::fmt::Display for SpeedUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpeedUnit::KmH => write!(f, "km/h"),
            SpeedUnit::Ms => write!(f, "m/s"),
            SpeedUnit::Mph => write!(f, "mph"),
        }
    }
}
