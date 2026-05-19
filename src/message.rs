use crate::carousel::CarouselId;
use crate::l10n::Locale;
use crate::media::MediaMetadata;
use crate::settings::{SpeedUnit, TemperatureUnit, ThemeMode};
use crate::weather::{GeoResult, WeatherStatus};
use crate::widgets::WidgetId;
use chrono::Utc;
use iced::window::Id;
use iced::{Color, Theme};
use std::time::Instant;
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager;

#[derive(Debug, Clone)]
pub enum Message {
    Tick(chrono::DateTime<Utc>),
    FetchWeather,
    WeatherFetched(WeatherStatus),
    OpenMainWindow,
    WindowOpened(Id),
    WindowClosed(Id),
    Quit,
    AnimateGradientC1(iced_anim::Event<Color>),
    AnimateGradientC2(iced_anim::Event<Color>),
    AnimateTheme(iced_anim::Event<Theme>),
    ThemeModeChanged(ThemeMode),
    FullscreenBtnHover(bool),
    SettingsBtnHover(bool),
    OpenSettings,
    CloseSettings,
    AnimateFullscreenBtn(iced_anim::Event<f32>),
    AnimateSettingsBtn(iced_anim::Event<f32>),
    WidgetHover(WidgetId, bool),
    WidgetAnimate(WidgetId, iced_anim::Event<f32>),
    OpenWidgetPreferences(WidgetId),
    CloseWidgetPreferences(WidgetId),
    ToggleFullscreen,
    ToggleSmoothTick(bool),
    ApplyTheme(ThemeMode),
    ThemeDarkAtChanged(String, bool),
    ThemeLightAtChanged(String, bool),
    ThemeAutoTick,
    DragDelta(f32),
    SnapTick(Instant),
    AnimTick(Instant),
    UpdatePageWidth(f32),
    GetPlayer,
    #[cfg(target_os = "windows")]
    PlayerInit(GlobalSystemMediaTransportControlsSessionManager),
    #[cfg(not(target_os = "windows"))]
    PlayerInit,
    MetadataSave(Option<MediaMetadata>),
    UpdateMetadata,
    Play,
    Pause,
    NextTrack,
    PreviousTrack,
    SeekPreview(f32),
    SeekCommit(f32),
    VolumePreview(f32),
    VolumeCommit(f32),
    VolumeGet,
    LocaleChanged(Locale),
    WidgetCityInputChanged(WidgetId, String),
    WidgetCitySearchResults(WidgetId, Vec<GeoResult>),
    WidgetCitySelected(WidgetId, GeoResult),
    WidgetWeatherFetched(WidgetId, WeatherStatus),
    TemperatureUnitChanged(TemperatureUnit),
    SpeedUnitChanged(SpeedUnit),
    CheckForUpdate,
    UpdateCheckResult(Option<String>),
    ApplyUpdate,
    UpdateApplied(Result<Option<String>, String>),
    CarouselChanged(CarouselId, usize),
    OpenUrl(String),
    None,
}
