pub mod calendar;
pub mod clock;
pub mod media;
pub mod weather;

use crate::l10n::L10n;
use crate::media::MediaMetadata;
use crate::message::Message;
use crate::settings::SpeedUnit;
use crate::weather::WeatherStatus;
use crate::widgets::calendar::CalendarWidget;
use crate::widgets::clock::ClockWidget;
use crate::widgets::media::MediaWidget;
use crate::widgets::weather::WeatherWidget;
use chrono::{DateTime, Utc};
use iced::{Color, Element, Size, Theme};

pub const WID_L0: WidgetId = WidgetId(0);
pub const WID_L1: WidgetId = WidgetId(1);
pub const WID_L2: WidgetId = WidgetId(2);
pub const WID_L3: WidgetId = WidgetId(3);
pub const WID_L4: WidgetId = WidgetId(4);
pub const WID_L5: WidgetId = WidgetId(5);
pub const WID_L6: WidgetId = WidgetId(6);
pub const WID_L7: WidgetId = WidgetId(7);
pub const WID_L8: WidgetId = WidgetId(8);

pub const WID_R0: WidgetId = WidgetId(8);
pub const WID_R1: WidgetId = WidgetId(9);
pub const WID_R2: WidgetId = WidgetId(10);
pub const WID_R3: WidgetId = WidgetId(11);
pub const WID_R4: WidgetId = WidgetId(12);
pub const WID_R5: WidgetId = WidgetId(13);

pub const WID_P0: WidgetId = WidgetId(14);
pub const WID_P1: WidgetId = WidgetId(15);
pub const WID_P2: WidgetId = WidgetId(16);

pub enum AppWidget {
    Calendar(CalendarWidget),
    Clock(ClockWidget),
    Weather(WeatherWidget),
    Media(MediaWidget),
}

pub trait ClearCache {
    fn clear_cache(&self);
}

impl AppWidget {
    pub fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        media_metadata: &'a Option<MediaMetadata>,
        size: Size,
        gc1: Color,
        gc2: Color,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
        smooth_tick: bool,
        l10n: &'a L10n,
        speed_unit: &'a SpeedUnit,
    ) -> Element<'a, Message> {
        match self {
            AppWidget::Clock(w) => w.view(time, weather, theme, size, smooth_tick, l10n),
            AppWidget::Calendar(w) => w.view(l10n, time),
            AppWidget::Weather(w) => w.view(theme, time, weather, size, l10n, speed_unit),
            AppWidget::Media(w) => w.view(
                media_metadata,
                theme,
                size,
                gc1,
                gc2,
                time,
                seek_preview,
                volume_preview,
                volume,
                l10n,
            ),
        }
    }

    pub fn clear_cache(&self) {
        match self {
            AppWidget::Clock(w) => w.clear_cache(),
            AppWidget::Calendar(w) => w.clear_cache(),
            AppWidget::Weather(w) => w.clear_cache(),
            AppWidget::Media(w) => w.clear_cache(),
        }
    }

    pub fn id(&self) -> WidgetId {
        match self {
            AppWidget::Calendar(w) => w.id,
            AppWidget::Clock(w) => w.id,
            AppWidget::Media(w) => w.id,
            AppWidget::Weather(w) => w.id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(pub usize);

fn arrow_svg(direction: &str) -> &'static [u8] {
    match direction {
        "up" => include_bytes!("../../icons/arrow-up-short.svg"),
        "down" => include_bytes!("../../icons/arrow-down-short.svg"),
        "repeat" => include_bytes!("../../icons/arrow-repeat.svg"),
        &_ => include_bytes!("../../icons/arrow-down-short.svg"),
    }
}
