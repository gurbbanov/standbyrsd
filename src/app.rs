use crate::carousel::{CarouselId, ease_spring, vertical_carousel};
use crate::config::{SavedCarousels, SavedConfig, SavedSettings, SavedWidgetPrefs};
use crate::l10n::L10n;
use crate::media::{MediaMetadata, extract_dominant_colors};
use crate::message::Message;
use crate::settings::{AppSettings, ThemeMode};
use crate::slide_pages::{DragState, slide_pages_func};
use crate::update::{apply_update, check_for_update};
use crate::weather::{GeoResponse, Weather, WeatherStatus};
use crate::widgets::{
    AppWidget, ClearCache, WID_L0, WID_L1, WID_L2, WID_L3, WID_L4, WID_L5, WID_L6, WID_L7, WID_L8,
    WID_P0, WID_P1, WID_P2, WID_R1, WID_R2, WID_R3, WID_R4, WID_R5, WidgetId,
};
use crate::widgets::{calendar::*, clock::*, media::*, weather::*};
use crate::{
    CURRENT_VERSION, FULLSCREEN_ENTER_SVG, FULLSCREEN_EXIT_SVG, IDLE_MS, PAGE_COUNT,
    SF_PRO_EXPANDED_BOLD, SNAP_DURATION_MS, SNAP_THRESHOLD,
};
use chrono::Utc;
use iced::theme::{Base, Palette};
use iced::time::{self, milliseconds, seconds};
use iced::widget::{
    button, column, combo_box, container, mouse_area, overlay, responsive, row, scrollable, stack,
    svg, text, text_input,
};
use iced::{
    Alignment, Color, Element, Length, Padding, Size, Subscription, Task, Theme, color,
    window::{self, Id},
};
use iced_anim::{Animated, Animation, Easing};
#[cfg(target_os = "macos")]
use media_remote;
use std::time::{Duration, Instant};
#[cfg(target_os = "windows")]
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

pub struct Application {
    pub time: chrono::DateTime<Utc>,
    pub weather: WeatherStatus,
    pub page0_left: Vec<AppWidget>,
    pub page0_right: Vec<AppWidget>,
    pub page1_widgets: Vec<AppWidget>,
    pub gradient_c1: Animated<Color>,
    pub gradient_c2: Animated<Color>,
    pub theme: Animated<Theme>,
    pub fullscreen: bool,
    pub fullscreen_btn_hover: Animated<f32>,
    pub settings_btn_hover: Animated<f32>,
    #[cfg(target_os = "windows")]
    pub playerctl: Option<GlobalSystemMediaTransportControlsSessionManager>,
    #[cfg(target_os = "windows")]
    pub session: Option<GlobalSystemMediaTransportControlsSession>,
    #[cfg(target_os = "macos")]
    pub now_playing: Option<media_remote::NowPlayingPerl>,
    pub media_metadata: Option<MediaMetadata>,
    pub seek_preview: Option<f32>,
    pub volume: f32,
    pub volume_preview: Option<f32>,
    pub main_window: Option<window::Id>,
    pub settings_open: bool,
    pub app_settings: AppSettings,
    pub current_page: usize,
    pub page_width: f32,
    pub drag: DragState,
    pub metadata_updating: bool,
    pub l10n: L10n,
    pub available_update: Option<String>,
    pub update_in_progress: bool,
    pub carousel_page0_left: usize,
    pub carousel_page0_right: usize,
    pub carousel_page1: usize,
}

impl Application {
    pub fn new() -> (Self, Task<Message>) {
        let cfg = SavedConfig::load();
        let mut app = Self::default();
        app.apply_config(&cfg);

        let locale = app.app_settings.locale.clone();
        let restore_weather: Vec<Task<Message>> = app
            .page0_left
            .iter()
            .chain(app.page0_right.iter())
            .chain(app.page1_widgets.iter())
            .filter_map(|w| match w {
                AppWidget::Clock(c) => c.selected_city.as_ref().map(|city| (c.id, city.clone())),
                AppWidget::Weather(w) => w.selected_city.as_ref().map(|city| (w.id, city.clone())),
                _ => None,
            })
            .map(|(id, city)| {
                let locale = locale.clone();
                Task::perform(
                    async move {
                        match Weather::fetch_for_city(&city, &locale).await {
                            Ok(w) => WeatherStatus::Ok(w),
                            Err(e) => WeatherStatus::Error(e.to_string()),
                        }
                    },
                    move |status| Message::WidgetWeatherFetched(id, status),
                )
            })
            .collect();

        (
            app,
            Task::batch([
                Task::done(Message::OpenMainWindow),
                Task::done(Message::LocaleChanged(locale)),
                Task::done(Message::ApplyTheme(cfg.settings.theme_mode)),
                Task::done(Message::GetPlayer),
                Task::done(Message::FetchWeather),
                Task::done(Message::CheckForUpdate),
                Task::batch(restore_weather),
            ]),
        )
    }

    fn build_config(&self) -> SavedConfig {
        let s = &self.app_settings;
        SavedConfig {
            settings: SavedSettings {
                theme_mode: s.theme_mode.clone(),
                theme_dark_at: s.theme_dark_at.clone(),
                theme_light_at: s.theme_light_at.clone(),
                smooth_tick: s.smooth_tick,
                locale: s.locale.clone(),
                temperature_unit: s.temperature_unit.clone(),
                speed_unit: s.speed_unit.clone(),
            },
            carousels: SavedCarousels {
                page0_left: self.carousel_page0_left,
                page0_right: self.carousel_page0_right,
                page1: self.carousel_page1,
            },
            widgets: self.collect_widget_prefs(),
        }
    }

    fn save_config(&self) {
        self.build_config().save()
    }

    fn collect_widget_prefs(&self) -> Vec<SavedWidgetPrefs> {
        self.page0_left
            .iter()
            .chain(self.page0_right.iter())
            .chain(self.page1_widgets.iter())
            .filter_map(|w| match w {
                AppWidget::Clock(c) => Some(SavedWidgetPrefs {
                    id: c.id.0,
                    selected_city: c.selected_city.clone(),
                    world_cities: if let ClockStyle::WorldHalf(world) = &c.style {
                        Some(world.tzs.clone())
                    } else {
                        None
                    },
                }),
                AppWidget::Weather(w) => Some(SavedWidgetPrefs {
                    id: w.id.0,
                    selected_city: w.selected_city.clone(),
                    world_cities: None,
                }),
                AppWidget::Calendar(_) | AppWidget::Media(_) => None,
            })
            .collect()
    }

    fn apply_config(&mut self, cfg: &SavedConfig) {
        self.app_settings.theme_mode = cfg.settings.theme_mode.clone();
        self.app_settings.theme_dark_at = cfg.settings.theme_dark_at.clone();
        self.app_settings.theme_light_at = cfg.settings.theme_light_at.clone();
        self.app_settings.smooth_tick = cfg.settings.smooth_tick;
        self.app_settings.locale = cfg.settings.locale.clone();
        self.app_settings.temperature_unit = cfg.settings.temperature_unit.clone();
        self.app_settings.speed_unit = cfg.settings.speed_unit.clone();

        self.carousel_page1 = cfg
            .carousels
            .page1
            .min(self.page1_widgets.len().saturating_sub(1));
        self.carousel_page0_left = cfg
            .carousels
            .page0_left
            .min(self.page0_left.len().saturating_sub(1));
        self.carousel_page0_right = cfg
            .carousels
            .page0_right
            .min(self.page0_right.len().saturating_sub(1));

        for w in self
            .page0_left
            .iter_mut()
            .chain(self.page0_right.iter_mut())
            .chain(self.page1_widgets.iter_mut())
        {
            match w {
                AppWidget::Clock(c) => {
                    if let Some(prefs) = cfg.widgets.iter().find(|p| p.id == c.id.0) {
                        c.selected_city = prefs.selected_city.clone();
                        if let (Some(cities), ClockStyle::WorldHalf(world)) =
                            (&prefs.world_cities, &mut c.style)
                        {
                            world.tzs = cities.clone();
                        }
                    }
                }
                AppWidget::Weather(w) => {
                    if let Some(prefs) = cfg.widgets.iter().find(|p| p.id == w.id.0) {
                        w.selected_city = prefs.selected_city.clone();
                    }
                }
                AppWidget::Calendar(_) | AppWidget::Media(_) => {}
            }
        }
    }

    fn try_snap(&mut self) {
        if let DragState::Active {
            offset_px,
            velocity,
            ..
        } = self.drag.clone()
        {
            let pw = self.page_width;
            let ratio = offset_px / pw;
            let from = self.current_page;
            let abs_now = -(from as f32) * pw + offset_px;

            let (target_page, abs_end) = if ratio < -SNAP_THRESHOLD && from + 1 < PAGE_COUNT {
                (from + 1, -((from + 1) as f32) * pw)
            } else if ratio > SNAP_THRESHOLD && from > 0 {
                (from - 1, -((from - 1) as f32) * pw)
            } else {
                (from, -(from as f32) * pw)
            };

            self.current_page = target_page;
            self.drag = DragState::Snapping {
                start_offset: abs_now,
                end_offset: abs_end,
                velocity,
                started_at: Instant::now(),
            };
        }
    }

    pub fn theme(&self, _id: Id) -> Theme {
        self.theme.value().clone()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(local_time) => {
                if local_time != self.time {
                    self.time = local_time;
                }
                Task::none()
            }
            Message::FetchWeather => {
                let locale = self.app_settings.locale.clone();
                let temp_unit = self.app_settings.temperature_unit.clone();
                let speed_unit = self.app_settings.speed_unit.clone();

                Task::perform(
                    async move {
                        let mut w = Weather::default();
                        match w.fetch(&locale, &temp_unit, &speed_unit).await {
                            Ok(()) => WeatherStatus::Ok(w),
                            Err(e) => WeatherStatus::Error(e.to_string()),
                        }
                    },
                    Message::WeatherFetched,
                )
            }
            Message::WeatherFetched(status) => {
                self.weather = status;

                for w in &self.page0_right {
                    w.clear_cache();
                }

                for w in &self.page1_widgets {
                    w.clear_cache();
                }

                Task::none()
            }
            Message::OpenMainWindow => {
                let (id, task) = window::open(window::Settings {
                    min_size: Some(Size {
                        width: 600.0,
                        height: 300.0,
                    }),
                    size: Size {
                        width: 800.0,
                        height: 600.0,
                    },
                    position: window::Position::Centered,
                    fullscreen: true,
                    ..Default::default()
                });

                self.main_window = Some(id);

                task.map(move |_| Message::WindowOpened(id))
            }
            Message::WindowOpened(id) => {
                self.main_window = Some(id);
                Task::none()
            }
            Message::WindowClosed(id) => {
                if Some(id) == self.main_window {
                    return Task::done(Message::Quit);
                }
                Task::none()
            }
            Message::Quit => iced::exit(),
            Message::GetPlayer => {
                #[cfg(target_os = "windows")]
                {
                    Task::perform(
                        async {
                            GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                                .unwrap()
                                .await
                                .unwrap()
                        },
                        Message::PlayerInit,
                    )
                }

                #[cfg(target_os = "linux")]
                {
                    Task::done(Message::PlayerInit)
                }

                #[cfg(target_os = "macos")]
                {
                    Task::done(Message::PlayerInit)
                }
            }
            #[cfg(target_os = "windows")]
            Message::PlayerInit(playerctl) => {
                self.playerctl = Some(playerctl.clone());

                let session = match playerctl.GetCurrentSession().ok() {
                    Some(s) => {
                        self.session = Some(s.clone());
                        s
                    }
                    None => return Task::none(),
                };

                let theme_name = self.theme.value().name().to_string();
                let (tx, rx) = tokio::sync::oneshot::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();

                    rt.block_on(async move {
                        let result: Option<MediaMetadata> = async {
                            let info = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;
                            let timeline = session.GetTimelineProperties().ok()?;
                            let playback = session.GetPlaybackInfo().ok()?;

                            let thumbnail_buf = async {
                                let stream = info.Thumbnail().ok()?.OpenReadAsync().ok()?.await.ok()?;
                                let size = stream.Size().ok()? as u32;
                                let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream).ok()?;
                                reader.LoadAsync(size).ok()?.await.ok()?;
                                let mut buf = vec![0u8; size as usize];
                                reader.ReadBytes(&mut buf).ok()?;
                                Some(buf)
                            }.await;

                            let gradient_colors = thumbnail_buf.as_ref().map(|buf| extract_dominant_colors(buf, &theme_name));
                            let thumbnail = thumbnail_buf.map(|buf| iced::widget::image::Handle::from_bytes(buf));

                            Some(MediaMetadata {
                                title: info.Title().ok()?.to_string(),
                                artist: info.Artist().ok()?.to_string(),
                                position: timeline.Position().ok()?.Duration,
                                duration: timeline.EndTime().ok()?.Duration,
                                is_playing: matches!(
                                    playback.PlaybackStatus().ok()?,
                                    windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                                ),
                                thumbnail,
                                gradient_colors,
                                position_origin: chrono::Utc::now(),
                            })
                        }.await;

                        let _ = tx.send(result);
                    });
                });

                Task::perform(
                    async move { rx.await.ok().flatten() },
                    Message::MetadataSave,
                )
            }
            #[cfg(target_os = "linux")]
            Message::PlayerInit => {
                let theme_name = self.theme.value().name().to_string();
                let (tx, rx) = tokio::sync::oneshot::channel();
                std::thread::spawn(move || {
                    let result: Option<MediaMetadata> = (|| {
                        let finder = mpris::PlayerFinder::new().ok()?;
                        let player = finder.find_active().ok()?;
                        let metadata = player.get_metadata().ok()?;

                        let playback = player.get_playback_status().ok()?;
                        let position = player
                            .get_position()
                            .ok()
                            .map(|p| p.as_micros() as i64 * 10)
                            .unwrap_or(0);
                        let duration = metadata
                            .length()
                            .map(|d| d.as_micros() as i64 * 10)
                            .unwrap_or(0);
                        let is_playing = matches!(playback, mpris::PlaybackStatus::Playing);

                        let title = metadata.title().unwrap_or("").to_string();
                        let artist = metadata
                            .artists()
                            .and_then(|a| a.first().cloned())
                            .unwrap_or("")
                            .to_string();

                        let thumbnail_buf = metadata
                            .get("mpris:artUrl")
                            .and_then(|v| v.as_str())
                            .and_then(|url| {
                                if url.starts_with("file://") {
                                    std::fs::read(url.trim_start_matches("file://")).ok()
                                } else if url.starts_with("http") {
                                    reqwest::blocking::get(url)
                                        .ok()
                                        .and_then(|r| r.bytes().ok())
                                        .map(|b| b.to_vec())
                                } else {
                                    None
                                }
                            });

                        let gradient_colors = thumbnail_buf
                            .as_ref()
                            .map(|buf| extract_dominant_colors(buf, &theme_name));
                        let thumbnail =
                            thumbnail_buf.map(|buf| iced::widget::image::Handle::from_bytes(buf));

                        Some(MediaMetadata {
                            title,
                            artist,
                            position,
                            duration,
                            is_playing,
                            thumbnail,
                            gradient_colors,
                            position_origin: chrono::Utc::now(),
                        })
                    })();

                    let _ = tx.send(result);
                });

                Task::perform(
                    async move { rx.await.ok().flatten() },
                    Message::MetadataSave,
                )
            }
            #[cfg(target_os = "macos")]
            Message::PlayerInit => {
                let now_playing = media_remote::NowPlayingPerl::new();
                self.now_playing = Some(now_playing);
                Task::perform(
                    async { tokio::time::sleep(Duration::from_millis(500)).await },
                    |_| Message::UpdateMetadata,
                )
            }
            #[cfg(target_os = "windows")]
            Message::UpdateMetadata => {
                if self.metadata_updating {
                    return Task::none();
                }

                self.metadata_updating = true;

                let session = match self.session.as_ref() {
                    Some(s) => s.clone(),
                    None => return Task::none(),
                };

                let theme_name = self.theme.value().name().to_string();
                let existing = self.media_metadata.clone();

                let (tx, rx) = tokio::sync::oneshot::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();

                    rt.block_on(async move {
                        let result: Option<MediaMetadata> = async {
                            let info = session.TryGetMediaPropertiesAsync().ok()?.await.ok()?;
                            let timeline = session.GetTimelineProperties().ok()?;
                            let playback = session.GetPlaybackInfo().ok()?;

                            let title = info.Title().ok()?.to_string();
                            let artist = info.Artist().ok()?.to_string();

                            let track_changed = existing.as_ref()
                                .map(|e| e.title != title || e.artist != artist)
                                .unwrap_or(true);

                            let (thumbnail, gradient_colors) = if track_changed {
                                let thumb = async {
                                    let stream = info.Thumbnail().ok()?.OpenReadAsync().ok()?.await.ok()?;
                                    let size = stream.Size().ok()? as u32;
                                    let reader = windows::Storage::Streams::DataReader::CreateDataReader(&stream).ok()?;
                                    reader.LoadAsync(size).ok()?.await.ok()?;
                                    let mut buf = vec![0u8; size as usize];
                                    reader.ReadBytes(&mut buf).ok()?;
                                    Some(buf)
                                }.await;

                                let gradient_colors = thumb.as_ref().map(|b| extract_dominant_colors(b, &theme_name));
                                let thumbnail = thumb.map(iced::widget::image::Handle::from_bytes);

                                (thumbnail, gradient_colors)
                            } else {
                                let e = existing.as_ref()?;
                                (e.thumbnail.clone(), e.gradient_colors)
                            };

                            let position = timeline.Position().ok()?.Duration;

                            let position_origin = if existing.as_ref().map(|e| e.position) == Some(position) {
                                existing.as_ref()?.position_origin
                            } else {
                                chrono::Utc::now()
                            };

                            Some(MediaMetadata {
                                title,
                                artist,
                                position,
                                duration: timeline.EndTime().ok()?.Duration,
                                is_playing: matches!(
                                    playback.PlaybackStatus().ok()?,
                                    windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                                ),
                                thumbnail,
                                gradient_colors,
                                position_origin
                            })
                        }.await;
                        let _ = tx.send(result);
                    });
                });

                Task::perform(
                    async move { rx.await.ok().flatten() },
                    Message::MetadataSave,
                )
            }
            #[cfg(target_os = "linux")]
            Message::UpdateMetadata => {
                if self.metadata_updating {
                    return Task::none();
                }

                self.metadata_updating = true;

                let existing = self.media_metadata.clone();
                let theme_name = self.theme.value().name().to_string();

                let (tx, rx) = tokio::sync::oneshot::channel();
                std::thread::spawn(move || {
                    let result: Option<MediaMetadata> = (|| {
                        let finder = mpris::PlayerFinder::new().ok()?;
                        let player = finder.find_active().ok()?;
                        let metadata = player.get_metadata().ok()?;

                        let playback = player.get_playback_status().ok()?;
                        let position = player
                            .get_position()
                            .ok()
                            .map(|p| p.as_micros() as i64 * 10)
                            .unwrap_or(0);
                        let duration = metadata
                            .length()
                            .map(|d| d.as_micros() as i64 * 10)
                            .unwrap_or(0);
                        let is_playing = matches!(playback, mpris::PlaybackStatus::Playing);

                        let title = metadata.title().unwrap_or("").to_string();
                        let artist = metadata
                            .artists()
                            .and_then(|a| a.first().cloned())
                            .unwrap_or("")
                            .to_string();

                        let title_changed =
                            existing.as_ref().map(|m| m.title.as_str()) != Some(title.as_str());

                        let position_origin =
                            if existing.as_ref().map(|e| e.position) == Some(position) {
                                existing.as_ref()?.position_origin
                            } else {
                                chrono::Utc::now()
                            };

                        if title_changed {
                            let thumbnail_buf = metadata
                                .get("mpris:artUrl")
                                .and_then(|v| v.as_str())
                                .and_then(|url| {
                                    if url.starts_with("file://") {
                                        std::fs::read(url.trim_start_matches("file://")).ok()
                                    } else if url.starts_with("http") {
                                        reqwest::blocking::get(url)
                                            .ok()
                                            .and_then(|r| r.bytes().ok())
                                            .map(|b| b.to_vec())
                                    } else {
                                        None
                                    }
                                });

                            let gradient_colors = thumbnail_buf
                                .as_ref()
                                .map(|buf| extract_dominant_colors(buf, &theme_name));
                            let thumbnail = thumbnail_buf
                                .map(|buf| iced::widget::image::Handle::from_bytes(buf));

                            return Some(MediaMetadata {
                                title,
                                artist,
                                position,
                                duration,
                                is_playing,
                                thumbnail,
                                gradient_colors,
                                position_origin,
                            });
                        } else {
                            Some(MediaMetadata {
                                title,
                                artist,
                                position,
                                duration,
                                is_playing,
                                position_origin,
                                ..existing?
                            })
                        }
                    })();

                    let _ = tx.send(result);
                });

                Task::perform(
                    async move { rx.await.ok().flatten() },
                    Message::MetadataSave,
                )
            }
            #[cfg(target_os = "macos")]
            Message::UpdateMetadata => {
                if self.metadata_updating {
                    return Task::none();
                }

                self.metadata_updating = true;

                let now_playing: Option<media_remote::NowPlayingInfo> = self
                    .now_playing
                    .as_ref()
                    .and_then(|np| np.get_info().as_ref().cloned());

                let existing = self.media_metadata.clone();
                let theme_name = self.theme.value().name().to_string();

                let (tx, rx) = tokio::sync::oneshot::channel();
                std::thread::spawn(move || {
                    let result: Option<MediaMetadata> = (|| {
                        let info = now_playing?;

                        let title = info.title.unwrap_or_default();
                        let artist = info.artist.unwrap_or_default();
                        let duration = (info.duration? * 1e7) as i64;
                        let position = (info.elapsed_time? * 1e7) as i64;
                        let is_playing = info.is_playing.unwrap_or(false);

                        let title_changed =
                            existing.as_ref().map(|e| e.title != title).unwrap_or(true);

                        let position_origin =
                            if existing.as_ref().map(|e| e.position) == Some(position) {
                                existing.as_ref()?.position_origin
                            } else {
                                chrono::Utc::now()
                            };

                        if title_changed {
                            let thumbnail_buf: Option<Vec<u8>> = info.album_cover.and_then(|img| {
                                let mut buf = std::io::Cursor::new(Vec::new());
                                img.write_to(&mut buf, image::ImageFormat::Png).ok()?;
                                Some(buf.into_inner())
                            });

                            let gradient_colors = thumbnail_buf
                                .as_ref()
                                .map(|buf| extract_dominant_colors(buf, &theme_name));
                            let thumbnail = thumbnail_buf
                                .map(|buf| iced::widget::image::Handle::from_bytes(buf));

                            Some(MediaMetadata {
                                title,
                                artist,
                                position,
                                duration,
                                is_playing,
                                thumbnail,
                                gradient_colors,
                                position_origin,
                            })
                        } else {
                            Some(MediaMetadata {
                                title,
                                artist,
                                position,
                                duration,
                                is_playing,
                                position_origin,
                                ..existing?
                            })
                        }
                    })();

                    let _ = tx.send(result);
                });

                Task::perform(
                    async move { rx.await.ok().flatten() },
                    Message::MetadataSave,
                )
            }
            Message::MetadataSave(metadata) => {
                self.metadata_updating = false;
                if let Some((c1, c2)) = metadata.as_ref().and_then(|m| m.gradient_colors) {
                    self.gradient_c1.set_target(c1);
                    self.gradient_c2.set_target(c2);
                }

                self.media_metadata = metadata;

                Task::none()
            }
            Message::ApplyTheme(mode) => {
                match mode {
                    ThemeMode::Classic => {
                        self.theme.update(iced_anim::Event::from(Theme::custom(
                            "classic".to_string(),
                            Palette {
                                text: Color::WHITE,
                                primary: color!(169, 169, 169),
                                danger: color!(87, 87, 87),
                                background: color!(0, 0, 0),
                                success: Color::WHITE,
                                warning: color!(240, 157, 10),
                                ..Theme::Moonfly.palette()
                            },
                        )));
                    }
                    ThemeMode::RedDark => {
                        self.theme.update(iced_anim::Event::from(Theme::custom(
                            "red_dark".to_string(),
                            Palette {
                                text: Color::from_rgb(1.0, 0.0, 0.0),
                                background: Color::from_rgb(0.0, 0.0, 0.0),
                                primary: color!(246, 0, 1),
                                success: color!(0, 0, 0),
                                warning: color!(159, 5, 0),
                                danger: color!(87, 4, 4),
                            },
                        )));
                    }
                    _ => {}
                }

                Task::done(Message::GetPlayer)
            }
            Message::ThemeModeChanged(mode) => {
                self.app_settings.theme_mode = mode.clone();

                match mode {
                    ThemeMode::Classic => Task::done(Message::ApplyTheme(ThemeMode::Classic)),
                    ThemeMode::RedDark => Task::done(Message::ApplyTheme(ThemeMode::RedDark)),
                    ThemeMode::AutoSunrise | ThemeMode::AutoCustom => Task::none(),
                }
            }
            Message::ThemeDarkAtChanged(s, is_hours) => {
                let filtered: String = s.chars().filter(|c| c.is_ascii_digit()).take(2).collect();

                let valid = if is_hours {
                    filtered.parse::<u8>().map_or(true, |v| v <= 23)
                } else {
                    filtered.parse::<u8>().map_or(true, |v| v <= 59)
                };

                if valid {
                    let parts: Vec<&str> = self.app_settings.theme_dark_at.split(':').collect();
                    let (h, m) = (parts.get(0).unwrap_or(&"22"), parts.get(1).unwrap_or(&"00"));
                    self.app_settings.theme_dark_at = if is_hours {
                        format!("{}:{}", filtered, m)
                    } else {
                        format!("{}:{}", h, filtered)
                    };
                }

                Task::none()
            }
            Message::ThemeLightAtChanged(s, is_hours) => {
                let filtered: String = s.chars().filter(|c| c.is_ascii_digit()).take(2).collect();

                let valid = if is_hours {
                    filtered.parse::<u8>().map_or(true, |v| v <= 23)
                } else {
                    filtered.parse::<u8>().map_or(true, |v| v <= 59)
                };

                if valid {
                    let parts: Vec<&str> = self.app_settings.theme_light_at.split(':').collect();
                    let (h, m) = (parts.get(0).unwrap_or(&"22"), parts.get(1).unwrap_or(&"00"));
                    self.app_settings.theme_light_at = if is_hours {
                        format!("{}:{}", filtered, m)
                    } else {
                        format!("{}:{}", h, filtered)
                    };
                }

                Task::none()
            }
            Message::ThemeAutoTick => {
                match self.app_settings.theme_mode {
                    ThemeMode::AutoCustom => {
                        if let (Ok(dark_at), Ok(light_at)) = (
                            chrono::NaiveTime::parse_from_str(
                                &self.app_settings.theme_dark_at,
                                "%H:%M",
                            ),
                            chrono::NaiveTime::parse_from_str(
                                &self.app_settings.theme_light_at,
                                "%H:%M",
                            ),
                        ) {
                            let now = self.time.time();

                            let should_be_dark = if dark_at < light_at {
                                now >= dark_at && now < light_at
                            } else {
                                now >= dark_at || now < light_at
                            };

                            let is_dark = self.theme.value().name() == "red_dark";

                            if should_be_dark != is_dark {
                                return Task::done(if should_be_dark {
                                    Message::ApplyTheme(ThemeMode::RedDark)
                                } else {
                                    Message::ApplyTheme(ThemeMode::Classic)
                                });
                            }
                        }
                    }
                    ThemeMode::AutoSunrise => {
                        if let WeatherStatus::Ok(w) = &self.weather {
                            if let Some(daily) = &w.daily {
                                if let (Some(sunrise), Some(sunset)) =
                                    (daily.sunrise.first(), daily.sunset.first())
                                {
                                    let sunrise_time = sunrise.split('T').nth(1).and_then(|t| {
                                        chrono::NaiveTime::parse_from_str(t, "%H:%M").ok()
                                    });

                                    let sunset_time = sunset.split('T').nth(1).and_then(|t| {
                                        chrono::NaiveTime::parse_from_str(t, "%H:%M").ok()
                                    });

                                    if let (Some(sunrise_time), Some(sunset_time)) =
                                        (sunrise_time, sunset_time)
                                    {
                                        let now = self.time.time();
                                        let should_be_dark =
                                            now < sunrise_time || now >= sunset_time;
                                        let is_dark = self.theme.value().name() == "red_dark";
                                        if should_be_dark != is_dark {
                                            return Task::done(if should_be_dark {
                                                Message::ApplyTheme(ThemeMode::RedDark)
                                            } else {
                                                Message::ApplyTheme(ThemeMode::Classic)
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::AnimateGradientC1(event) => {
                self.gradient_c1.update(event);

                Task::none()
            }
            Message::AnimateGradientC2(event) => {
                self.gradient_c2.update(event);

                Task::none()
            }
            Message::AnimateTheme(event) => {
                self.theme.update(event);
                for w in &self.page0_left {
                    w.clear_cache();
                }

                for w in &self.page0_right {
                    w.clear_cache();
                }

                for w in &self.page1_widgets {
                    w.clear_cache();
                }

                Task::none()
            }
            Message::ToggleFullscreen => {
                if let Some(id) = self.main_window {
                    self.fullscreen = !self.fullscreen;

                    if self.fullscreen {
                        window::set_mode::<Message>(id, window::Mode::Fullscreen).into()
                    } else {
                        window::set_mode::<Message>(id, window::Mode::Windowed).into()
                    }
                } else {
                    Task::none()
                }
            }
            Message::ToggleSmoothTick(b) => {
                self.app_settings.smooth_tick = b;

                Task::none()
            }
            Message::FullscreenBtnHover(hovered) => {
                self.fullscreen_btn_hover
                    .set_target(if hovered { 1.0 } else { 0.0 });

                Task::none()
            }
            Message::SettingsBtnHover(hovered) => {
                self.settings_btn_hover
                    .set_target(if hovered { 1.0 } else { 0.0 });

                Task::none()
            }
            Message::AnimateFullscreenBtn(e) => {
                self.fullscreen_btn_hover.update(e);

                Task::none()
            }
            Message::AnimateSettingsBtn(e) => {
                self.settings_btn_hover.update(e);

                Task::none()
            }
            Message::WidgetHover(id, hovered) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => {
                        w.hover.set_target(if hovered { 1.0 } else { 0.0 })
                    }
                    Some(AppWidget::Weather(w)) => {
                        w.hover.set_target(if hovered { 1.0 } else { 0.0 })
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::WidgetAnimate(id, event) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.hover.update(event),
                    Some(AppWidget::Weather(w)) => w.hover.update(event),
                    _ => {}
                }

                Task::none()
            }
            Message::DragDelta(dx) => {
                let pw = self.page_width;
                let prev = match &self.drag {
                    DragState::Active { offset_px, .. } => *offset_px,
                    DragState::Snapping {
                        start_offset,
                        end_offset,
                        velocity,
                        started_at,
                    } => {
                        let elapsed = started_at.elapsed().as_secs_f32();
                        let t = (elapsed / (SNAP_DURATION_MS as f32 / 1000.0)).min(1.0);
                        let dist = end_offset - start_offset;
                        let v0 = if dist.abs() > 0.001 {
                            velocity / dist
                        } else {
                            0.0
                        };
                        let abs = start_offset + dist * ease_spring(t, v0);
                        abs - (-(self.current_page as f32) * pw)
                    }
                    DragState::Idle => 0.0,
                };
                let raw = prev + dx;
                let max_drag = if self.current_page > 0 { pw } else { 0.0 };
                let min_drag = if self.current_page + 1 < PAGE_COUNT {
                    -pw
                } else {
                    0.0
                };
                let clamped = raw.clamp(min_drag, max_drag);
                self.drag = DragState::Active {
                    offset_px: clamped,
                    velocity: dx,
                    last_event: Instant::now(),
                };

                if dx.abs() < 1.5 {
                    self.try_snap();
                }

                Task::none()
            }
            Message::SnapTick(_) => {
                if let DragState::Active { last_event, .. } = self.drag.clone() {
                    if last_event.elapsed() >= Duration::from_millis(IDLE_MS) {
                        self.try_snap();
                    }
                }

                Task::none()
            }
            Message::AnimTick(_) => {
                if self.drag.is_snapping_done() {
                    self.drag = DragState::Idle;
                }

                Task::none()
            }
            Message::UpdatePageWidth(w) => {
                self.page_width = w;

                Task::none()
            }
            Message::Play => {
                #[cfg(target_os = "windows")]
                {
                    let session = self.session.clone();
                    Task::perform(
                        async move {
                            if let Some(s) = session {
                                s.TryPlayAsync().unwrap().await.unwrap();
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "linux")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let finder = mpris::PlayerFinder::new().ok()?;
                                let player = finder.find_active().ok()?;
                                player.play().ok()?;
                                Some(())
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "macos")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(|| {
                                media_remote::send_command(media_remote::Command::Play);
                            })
                            .await
                            .ok();
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }
            }
            Message::Pause => {
                #[cfg(target_os = "windows")]
                {
                    let session = self.session.clone();
                    Task::perform(
                        async move {
                            if let Some(s) = session {
                                s.TryPauseAsync().unwrap().await.unwrap();
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "linux")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let finder = mpris::PlayerFinder::new().ok()?;
                                let player = finder.find_active().ok()?;
                                player.pause().ok()?;
                                Some(())
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "macos")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(|| {
                                media_remote::send_command(media_remote::Command::Pause);
                            })
                            .await
                            .ok();
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }
            }
            Message::NextTrack => {
                #[cfg(target_os = "windows")]
                {
                    let session = self.session.clone();
                    Task::perform(
                        async move {
                            if let Some(s) = session {
                                s.TrySkipNextAsync().unwrap().await.unwrap();
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "linux")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let finder = mpris::PlayerFinder::new().ok()?;
                                let player = finder.find_active().ok()?;
                                player.next().ok()?;
                                Some(())
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "macos")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(|| {
                                media_remote::send_command(media_remote::Command::NextTrack);
                            })
                            .await
                            .ok();
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }
            }
            Message::PreviousTrack => {
                #[cfg(target_os = "windows")]
                {
                    let session = self.session.clone();
                    Task::perform(
                        async move {
                            if let Some(s) = session {
                                s.TrySkipPreviousAsync().unwrap().await.unwrap();
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "linux")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let finder = mpris::PlayerFinder::new().ok()?;
                                let player = finder.find_active().ok()?;
                                player.previous().ok()?;
                                Some(())
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |_| Message::UpdateMetadata,
                    )
                }

                #[cfg(target_os = "macos")]
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(|| {
                                media_remote::send_command(media_remote::Command::PreviousTrack);
                            })
                            .await
                            .ok();
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    )
                }
            }
            Message::SeekPreview(ratio) => {
                self.seek_preview = Some(ratio);

                Task::none()
            }
            Message::SeekCommit(ratio) => {
                self.seek_preview = None;
                #[cfg(target_os = "windows")]
                {
                    let session = self.session.clone();
                    let duration = self
                        .media_metadata
                        .as_ref()
                        .map(|m| m.duration)
                        .unwrap_or(0);
                    let position = (ratio * duration as f32) as i64;
                    return Task::perform(
                        async move {
                            if let Some(s) = session {
                                use windows::Foundation::TimeSpan;
                                s.TryChangePlaybackPositionAsync(
                                    TimeSpan { Duration: position }.Duration,
                                )
                                .unwrap()
                                .await
                                .unwrap();
                            }
                            tokio::time::sleep(Duration::from_millis(300)).await;
                        },
                        |_| Message::UpdateMetadata,
                    );
                }
                #[cfg(target_os = "linux")]
                {
                    let duration_us = self
                        .media_metadata
                        .as_ref()
                        .map(|m| m.duration)
                        .unwrap_or(0)
                        / 10;

                    let target_us = (ratio * duration_us as f32) as i64;

                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                let finder = mpris::PlayerFinder::new().ok()?;
                                let player = finder.find_active().ok()?;

                                let current_us = player.get_position().ok()?.as_micros() as i64;
                                let delta_us = target_us - current_us;

                                player.seek(delta_us).ok()?;
                                Some(())
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |_| Message::UpdateMetadata,
                    );
                }
                #[cfg(target_os = "macos")]
                {
                    let duration = self
                        .media_metadata
                        .as_ref()
                        .map(|m| m.duration)
                        .unwrap_or(0);
                    let target_secs = ratio as f64 * duration as f64 / 10_000_000.0;
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                media_remote::set_elapsed_time(target_secs);
                            })
                            .await
                            .ok();
                        },
                        |_| Message::UpdateMetadata,
                    );
                }
            }
            Message::VolumePreview(v) => {
                self.volume_preview = Some(v);

                Task::none()
            }
            Message::VolumeCommit(v) => {
                std::thread::spawn(move || {
                    if let Ok(device) = volumecontrol::AudioDevice::from_default() {
                        if device.set_vol((v * 100.0) as u8).is_ok() {
                            return;
                        }
                    }
                    #[cfg(target_os = "macos")]
                    {
                        let vol = (v * 100.0) as u8;
                        std::process::Command::new("osascript")
                            .arg("-e")
                            .arg(format!("set volume output volume {vol}"))
                            .output()
                            .ok();
                    }
                });

                Task::none()
            }
            Message::VolumeGet => {
                if self.volume_preview.is_some() {
                    return Task::none();
                }
                if let Ok(device) = volumecontrol::AudioDevice::from_default() {
                    if let Ok(vol) = device.get_vol() {
                        self.volume = vol as f32 / 100.0;
                    }
                }

                Task::none()
            }
            Message::OpenSettings => {
                self.settings_open = true;

                Task::none()
            }
            Message::CloseSettings => {
                self.settings_open = false;
                self.save_config();

                Task::none()
            }
            Message::OpenWidgetPreferences(id) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.preferences_open = true,
                    Some(AppWidget::Weather(w)) => w.preferences_open = true,
                    _ => {}
                }

                Task::none()
            }
            Message::CloseWidgetPreferences(id) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.preferences_open = false,
                    Some(AppWidget::Weather(w)) => w.preferences_open = false,
                    _ => {}
                }
                self.save_config();

                Task::none()
            }
            Message::LocaleChanged(locale) => {
                self.app_settings.locale = locale.clone();
                self.l10n = L10n::new(self.app_settings.locale.as_str());

                Task::done(Message::FetchWeather)
            }
            Message::WidgetCityInputChanged(id, input) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.city_input = input.clone(),
                    Some(AppWidget::Weather(w)) => w.city_input = input.clone(),
                    _ => {}
                }
                let locale = self.app_settings.locale.clone();
                Task::perform(
                    async move {
                        reqwest::get(format!(
                            "https://geocoding-api.open-meteo.com/v1/search?name={}&language={}&count=5",
                            input, locale.as_str()
                        ))
                        .await?
                        .json::<GeoResponse>()
                        .await
                        .map(|r| r.results.unwrap_or_default())
                    },
                    move |res| Message::WidgetCitySearchResults(id, res.unwrap_or_default()),
                )
            }
            Message::WidgetCitySearchResults(id, results) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.city_results = results,
                    Some(AppWidget::Weather(w)) => w.city_results = results,
                    _ => {}
                }

                Task::none()
            }
            Message::WidgetCitySelected(id, city) => {
                let locale = self.app_settings.locale.clone();
                let city_clone = city.clone();
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => {
                        w.selected_city = Some(city.clone());
                        w.custom_weather = Some(WeatherStatus::Loading);
                        w.city_results = vec![];
                        w.city_input = city.name.clone();
                    }
                    Some(AppWidget::Weather(w)) => {
                        w.selected_city = Some(city.clone());
                        w.custom_weather = Some(WeatherStatus::Loading);
                        w.city_results = vec![];
                        w.city_input = city.name.clone();
                    }
                    _ => {}
                }

                Task::perform(
                    async move {
                        match Weather::fetch_for_city(&city_clone, &locale).await {
                            Ok(w) => WeatherStatus::Ok(w),
                            Err(e) => WeatherStatus::Error(e.to_string()),
                        }
                    },
                    move |status| Message::WidgetWeatherFetched(id, status),
                )
            }
            Message::WidgetWeatherFetched(id, status) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => {
                        w.custom_weather = Some(status);
                        w.clear_cache();
                    }
                    Some(AppWidget::Weather(w)) => {
                        w.custom_weather = Some(status);
                        w.clear_cache();
                    }
                    _ => {}
                }

                Task::none()
            }
            Message::WorldCityInputChanged(id, index, input) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.world_city_inputs[index] = input.clone(),
                    _ => {}
                }
                let locale = self.app_settings.locale.clone();
                Task::perform(
                    async move {
                        reqwest::get(format!(
                            "https://geocoding-api.open-meteo.com/v1/search?name={}&language={}&count=5",
                            input, locale.as_str()
                        ))
                        .await?
                        .json::<GeoResponse>()
                        .await
                        .map(|r| r.results.unwrap_or_default())
                    },
                    move |res| Message::WorldCitySearchResults(id, index, res.unwrap_or_default()),
                )
            }
            Message::WorldCitySearchResults(id, index, results) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => w.world_city_results[index] = results,
                    _ => {}
                }
                Task::none()
            }
            Message::WorldCitySelected(id, index, city) => {
                match self.find_widget_mut(id) {
                    Some(AppWidget::Clock(w)) => {
                        if let ClockStyle::WorldHalf(world) = &mut w.style {
                            world.tzs[index] = Some(city.clone());
                        }
                        w.world_city_results[index] = vec![];
                        w.world_city_inputs[index] = city.name.clone();
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::TemperatureUnitChanged(temp_unit) => {
                self.app_settings.temperature_unit = temp_unit.clone();

                Task::done(Message::FetchWeather)
            }
            Message::SpeedUnitChanged(speed_unit) => {
                self.app_settings.speed_unit = speed_unit.clone();

                Task::done(Message::FetchWeather)
            }
            Message::CheckForUpdate => Task::perform(
                async {
                    tokio::task::spawn_blocking(|| check_for_update().ok().flatten())
                        .await
                        .ok()
                        .flatten()
                },
                Message::UpdateCheckResult,
            ),
            Message::UpdateCheckResult(version) => {
                self.available_update = version;

                Task::none()
            }
            Message::ApplyUpdate => {
                self.update_in_progress = true;
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| apply_update().map_err(|e| e.to_string()))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    Message::UpdateApplied,
                )
            }
            Message::UpdateApplied(result) => {
                self.update_in_progress = false;

                if let Ok(Some(_ver)) = result {
                    std::process::Command::new(std::env::current_exe().unwrap())
                        .spawn()
                        .ok();
                    std::process::exit(0);
                }

                Task::none()
            }
            Message::CarouselChanged(carousel_id, index) => {
                match carousel_id {
                    CarouselId::Page0Left => self.carousel_page0_left = index,
                    CarouselId::Page0Right => self.carousel_page0_right = index,
                    CarouselId::Page1 => self.carousel_page1 = index,
                }
                self.save_config();

                Task::none()
            }
            Message::OpenUrl(url) => {
                let _ = open::that(url);
                Task::none()
            }
            Message::None => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let clock = time::every(if self.app_settings.smooth_tick {
            milliseconds(16)
        } else {
            seconds(1)
        })
        .map(|_| Message::Tick(chrono::Utc::now()));

        let weather = time::every(seconds(600)).map(|_| Message::FetchWeather);

        let metadata_update = time::every(seconds(1)).map(|_| Message::UpdateMetadata);

        let snap_idle = if matches!(self.drag, DragState::Active { .. }) {
            time::every(Duration::from_millis(16)).map(Message::SnapTick)
        } else {
            Subscription::none()
        };

        let anim = if matches!(self.drag, DragState::Snapping { .. }) {
            time::every(Duration::from_millis(16)).map(Message::AnimTick)
        } else {
            Subscription::none()
        };

        let theme = match self.app_settings.theme_mode {
            ThemeMode::AutoSunrise | ThemeMode::AutoCustom => {
                time::every(Duration::from_mins(2)).map(|_| Message::ThemeAutoTick)
            }
            _ => Subscription::none(),
        };

        let window_close = window::close_events().map(Message::WindowClosed);

        let volume_get = time::every(seconds(2)).map(|_| Message::VolumeGet);

        Subscription::batch([
            clock,
            weather,
            metadata_update,
            snap_idle,
            anim,
            theme,
            window_close,
            volume_get,
        ])
    }

    pub fn view(&self, id: Id) -> Element<'_, Message> {
        if Some(id) == self.main_window {
            let main_window = Animation::new(
                &self.theme,
                Animation::new(
                    &self.fullscreen_btn_hover,
                    Animation::new(
                        &self.settings_btn_hover,
                        Animation::new(
                            &self.gradient_c1,
                            Animation::new(
                                &self.gradient_c2,
                                responsive(move |size| {
                                    let total_offset: f32 = match &self.drag {
                                        DragState::Idle => -(self.current_page as f32) * size.width,
                                        DragState::Active { offset_px, .. } => {
                                            -(self.current_page as f32) * size.width + offset_px
                                        }
                                        DragState::Snapping {
                                            start_offset,
                                            end_offset,
                                            velocity,
                                            started_at,
                                        } => {
                                            let elapsed = started_at.elapsed().as_secs_f32();
                                            let t = (elapsed / (SNAP_DURATION_MS as f32 / 1000.0))
                                                .min(1.0);
                                            let dist = end_offset - start_offset;
                                            let v0 = if dist.abs() > 0.001 {
                                                velocity / dist
                                            } else {
                                                0.0
                                            };
                                            start_offset + dist * ease_spring(t, v0)
                                        }
                                    };

                                    slide_pages_func(
                                        total_offset,
                                        size.width,
                                        size.height,
                                        self.page0(size),
                                        self.page1(size),
                                    )
                                }),
                            )
                            .on_update(Message::AnimateGradientC2),
                        )
                        .on_update(Message::AnimateGradientC1),
                    )
                    .on_update(Message::AnimateSettingsBtn),
                )
                .on_update(Message::AnimateFullscreenBtn),
            )
            .on_update(Message::AnimateTheme);

            let theme = self.theme.value();

            stack![
                main_window,
                mouse_area(if self.settings_open {
                    container(
                        mouse_area(responsive(move |s| {
                            let mn = s.height.min(s.width);
                            container(
                                container(
                                    column![
                                        row![
                                            container(
                                                text(self.l10n.get("settings"))
                                                    .size(mn * 0.05)
                                                    .color(theme.palette().text)
                                            )
                                            .width(Length::Fill)
                                            .align_x(iced::Alignment::Start),
                                            container(
                                                button(
                                                    container("")
                                                        .width(Length::Fixed(mn * 0.02))
                                                        .height(Length::Fixed(mn * 0.02))
                                                )
                                                .on_press(Message::CloseSettings)
                                                .width(Length::Fixed(mn * 0.02))
                                                .height(Length::Fixed(mn * 0.02))
                                                .padding(0)
                                                .style(|_, status| {
                                                    let color = match status {
                                                        button::Status::Hovered => {
                                                            Color::from_rgb8(255, 80, 80)
                                                        }
                                                        _ => Color::from_rgb8(220, 50, 50),
                                                    };

                                                    button::Style {
                                                        background: Some(iced::Background::Color(
                                                            color,
                                                        )),
                                                        border: iced::Border {
                                                            radius: 10.0.into(),
                                                            ..Default::default()
                                                        },
                                                        ..Default::default()
                                                    }
                                                })
                                            )
                                            .width(Length::Shrink)
                                            .align_x(iced::Alignment::End),
                                        ]
                                        .width(Length::Fill),
                                        scrollable(
                                            column![
                                                column![
                                                    row![
                                                        container(
                                                            text(self.l10n.get("theme"))
                                                                .size(mn * 0.022)
                                                                .color(theme.palette().text)
                                                        )
                                                        .width(Length::Fill)
                                                        .align_x(iced::Alignment::Start),
                                                        container(
                                                            combo_box(
                                                                &self.app_settings.theme_mode_combo,
                                                                &self.l10n.get("select-theme"),
                                                                Some(&self.app_settings.theme_mode),
                                                                Message::ThemeModeChanged,
                                                            )
                                                            .width(Length::Fixed(mn * 0.24))
                                                            .input_style(move |_t, _status| {
                                                                text_input::Style {
                                                                    value: theme.palette().text,
                                                                    placeholder: theme
                                                                        .palette()
                                                                        .text,
                                                                    selection: theme
                                                                        .palette()
                                                                        .primary,
                                                                    background:
                                                                        iced::Background::Color(
                                                                            Color::TRANSPARENT,
                                                                        ),
                                                                    border: iced::Border {
                                                                        color: theme
                                                                            .palette()
                                                                            .primary,
                                                                        width: 1.0,
                                                                        radius: 4.0.into(),
                                                                    },
                                                                    icon: theme.palette().text,
                                                                }
                                                            })
                                                            .menu_style(move |_t| {
                                                                overlay::menu::Style {
                                                                    text_color: theme
                                                                        .palette()
                                                                        .text,
                                                                    background:
                                                                        iced::Background::Color(
                                                                            Color::BLACK,
                                                                        ),
                                                                    border: iced::Border {
                                                                        color: theme
                                                                            .palette()
                                                                            .primary,
                                                                        width: 1.0,
                                                                        radius: 4.0.into(),
                                                                    },
                                                                    selected_text_color:
                                                                        Color::BLACK,
                                                                    selected_background:
                                                                        iced::Background::Color(
                                                                            theme.palette().primary,
                                                                        ),
                                                                    shadow: iced::Shadow::default(),
                                                                }
                                                            })
                                                            .size(mn * 0.02)
                                                        )
                                                        .align_x(iced::Alignment::End)
                                                    ],
                                                    if self.app_settings.theme_mode
                                                        == ThemeMode::AutoCustom
                                                    {
                                                        row![
                                                            container(row![
                                                                container(
                                                                    text(self.l10n.get("light-at"))
                                                                        .size(mn * 0.022)
                                                                        .color(
                                                                            theme.palette().text
                                                                        )
                                                                )
                                                                .width(Length::Fill)
                                                                .align_x(iced::Alignment::Start),
                                                                container(row![
                                                            text_input(
                                                                "22",
                                                                &self
                                                                    .app_settings
                                                                    .theme_light_at
                                                                    .split(':')
                                                                    .next()
                                                                    .unwrap_or("00")
                                                            )
                                                            .size(mn * 0.02)
                                                            .on_input(|s| {
                                                                Message::ThemeLightAtChanged(
                                                                    s, true,
                                                                )
                                                            })
                                                            .width(Length::Fixed(mn * 0.05))
                                                            .style(move |_t, _status| {
                                                                text_input::Style {
                                                                    value: theme.palette().text,
                                                                    placeholder: theme
                                                                        .palette()
                                                                        .text,
                                                                    selection: theme
                                                                        .palette()
                                                                        .danger,
                                                                    background:
                                                                        iced::Background::Color(
                                                                            Color::TRANSPARENT,
                                                                        ),
                                                                    border: iced::Border {
                                                                        color: theme
                                                                            .palette()
                                                                            .primary,
                                                                        width: 1.0,
                                                                        radius: 4.0.into(),
                                                                    },
                                                                    icon: theme.palette().text,
                                                                }
                                                            }),
                                                            text(":")
                                                                .size(mn * 0.022)
                                                                .color(theme.palette().text),
                                                            text_input(
                                                                "00",
                                                                &self
                                                                    .app_settings
                                                                    .theme_light_at
                                                                    .split(':')
                                                                    .nth(1)
                                                                    .unwrap_or("00")
                                                            )
                                                            .size(mn * 0.02)
                                                            .on_input(|s| {
                                                                Message::ThemeLightAtChanged(
                                                                    s, false,
                                                                )
                                                            })
                                                            .width(Length::Fixed(mn * 0.05))
                                                            .style(move |_t, _status| {
                                                                text_input::Style {
                                                                    value: theme.palette().text,
                                                                    placeholder: theme
                                                                        .palette()
                                                                        .text,
                                                                    selection: theme
                                                                        .palette()
                                                                        .danger,
                                                                    background:
                                                                        iced::Background::Color(
                                                                            Color::TRANSPARENT,
                                                                        ),
                                                                    border: iced::Border {
                                                                        color: theme
                                                                            .palette()
                                                                            .primary,
                                                                        width: 1.0,
                                                                        radius: 4.0.into(),
                                                                    },
                                                                    icon: theme.palette().text,
                                                                }
                                                            })
                                                        ])
                                                                .align_x(iced::Alignment::End)
                                                            ])
                                                            .align_x(iced::Alignment::End)
                                                        ]
                                                    } else {
                                                        row![]
                                                    },
                                                    if self.app_settings.theme_mode
                                                        == ThemeMode::AutoCustom
                                                    {
                                                        row![
                                                            container(
                                                                text(self.l10n.get("dark-at"))
                                                                    .size(mn * 0.022)
                                                                    .color(theme.palette().text)
                                                            )
                                                            .width(Length::Fill)
                                                            .align_x(iced::Alignment::Start),
                                                            container(row![
                                                                text_input(
                                                                    "22",
                                                                    &self
                                                                        .app_settings
                                                                        .theme_dark_at
                                                                        .split(':')
                                                                        .next()
                                                                        .unwrap_or("00")
                                                                )
                                                                .size(mn * 0.02)
                                                                .on_input(|s| {
                                                                    Message::ThemeDarkAtChanged(
                                                                        s, true,
                                                                    )
                                                                })
                                                                .width(Length::Fixed(mn * 0.05))
                                                                .style(move |_t, _status| {
                                                                    text_input::Style {
                                                                        value: theme.palette().text,
                                                                        placeholder: theme
                                                                            .palette()
                                                                            .text,
                                                                        selection: theme
                                                                            .palette()
                                                                            .danger,
                                                                        background:
                                                                            iced::Background::Color(
                                                                                Color::TRANSPARENT,
                                                                            ),
                                                                        border: iced::Border {
                                                                            color: theme
                                                                                .palette()
                                                                                .primary,
                                                                            width: 1.0,
                                                                            radius: 4.0.into(),
                                                                        },
                                                                        icon: theme.palette().text,
                                                                    }
                                                                }),
                                                                text(":")
                                                                    .size(mn * 0.022)
                                                                    .color(theme.palette().text),
                                                                text_input(
                                                                    "00",
                                                                    &self
                                                                        .app_settings
                                                                        .theme_dark_at
                                                                        .split(':')
                                                                        .nth(1)
                                                                        .unwrap_or("00")
                                                                )
                                                                .size(mn * 0.02)
                                                                .on_input(|s| {
                                                                    Message::ThemeDarkAtChanged(
                                                                        s, false,
                                                                    )
                                                                })
                                                                .width(Length::Fixed(mn * 0.05))
                                                                .style(move |_t, _status| {
                                                                    text_input::Style {
                                                                        value: theme.palette().text,
                                                                        placeholder: theme
                                                                            .palette()
                                                                            .text,
                                                                        selection: theme
                                                                            .palette()
                                                                            .danger,
                                                                        background:
                                                                            iced::Background::Color(
                                                                                Color::TRANSPARENT,
                                                                            ),
                                                                        border: iced::Border {
                                                                            color: theme
                                                                                .palette()
                                                                                .primary,
                                                                            width: 1.0,
                                                                            radius: 4.0.into(),
                                                                        },
                                                                        icon: theme.palette().text,
                                                                    }
                                                                }),
                                                            ])
                                                            .align_x(iced::Alignment::End)
                                                        ]
                                                    } else {
                                                        row![]
                                                    },
                                                ]
                                                .width(Length::Fill)
                                                .spacing(mn * 0.01),
                                                row![
                                                    container(
                                                        text(self.l10n.get("smooth-tick"))
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(
                                                        iced::widget::toggler(
                                                            self.app_settings.smooth_tick
                                                        )
                                                        .size(mn * 0.025)
                                                        .on_toggle(Message::ToggleSmoothTick)
                                                    )
                                                    .align_x(iced::Alignment::End)
                                                ],
                                                row![
                                                    container(
                                                        text(self.l10n.get("language"))
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(
                                                        combo_box(
                                                            &self.app_settings.locale_combo,
                                                            &self.l10n.get("select-language"),
                                                            Some(&self.app_settings.locale),
                                                            Message::LocaleChanged,
                                                        )
                                                        .width(Length::Fixed(mn * 0.2))
                                                        .input_style(move |_t, _status| {
                                                            text_input::Style {
                                                                value: theme.palette().text,
                                                                placeholder: theme.palette().text,
                                                                selection: theme.palette().primary,
                                                                background: iced::Background::Color(
                                                                    Color::TRANSPARENT,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                icon: theme.palette().text,
                                                            }
                                                        })
                                                        .menu_style(move |_t| {
                                                            overlay::menu::Style {
                                                                text_color: theme.palette().text,
                                                                background: iced::Background::Color(
                                                                    Color::BLACK,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                selected_text_color: Color::BLACK,
                                                                selected_background:
                                                                    iced::Background::Color(
                                                                        theme.palette().primary,
                                                                    ),
                                                                shadow: iced::Shadow::default(),
                                                            }
                                                        })
                                                        .size(mn * 0.02)
                                                    )
                                                    .align_x(iced::Alignment::End)
                                                ],
                                                row![
                                                    container(
                                                        text(self.l10n.get("temperature-unit"))
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(
                                                        combo_box(
                                                            &self.app_settings.temperature_combo,
                                                            &self.l10n.get("select-unit"),
                                                            Some(
                                                                &self.app_settings.temperature_unit
                                                            ),
                                                            Message::TemperatureUnitChanged,
                                                        )
                                                        .width(Length::Fixed(mn * 0.24))
                                                        .input_style(move |_t, _status| {
                                                            text_input::Style {
                                                                value: theme.palette().text,
                                                                placeholder: theme.palette().text,
                                                                selection: theme.palette().primary,
                                                                background: iced::Background::Color(
                                                                    Color::TRANSPARENT,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                icon: theme.palette().text,
                                                            }
                                                        })
                                                        .menu_style(move |_t| {
                                                            overlay::menu::Style {
                                                                text_color: theme.palette().text,
                                                                background: iced::Background::Color(
                                                                    Color::BLACK,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                selected_text_color: Color::BLACK,
                                                                selected_background:
                                                                    iced::Background::Color(
                                                                        theme.palette().primary,
                                                                    ),
                                                                shadow: iced::Shadow::default(),
                                                            }
                                                        })
                                                        .size(mn * 0.02)
                                                    )
                                                    .align_x(iced::Alignment::End)
                                                ],
                                                row![
                                                    container(
                                                        text(self.l10n.get("speed-unit"))
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(
                                                        combo_box(
                                                            &self.app_settings.speed_combo,
                                                            &self.l10n.get("select-unit"),
                                                            Some(&self.app_settings.speed_unit),
                                                            Message::SpeedUnitChanged,
                                                        )
                                                        .width(Length::Fixed(mn * 0.24))
                                                        .input_style(move |_t, _status| {
                                                            text_input::Style {
                                                                value: theme.palette().text,
                                                                placeholder: theme.palette().text,
                                                                selection: theme.palette().primary,
                                                                background: iced::Background::Color(
                                                                    Color::TRANSPARENT,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                icon: theme.palette().text,
                                                            }
                                                        })
                                                        .menu_style(move |_t| {
                                                            overlay::menu::Style {
                                                                text_color: theme.palette().text,
                                                                background: iced::Background::Color(
                                                                    Color::BLACK,
                                                                ),
                                                                border: iced::Border {
                                                                    color: theme.palette().primary,
                                                                    width: 1.0,
                                                                    radius: 4.0.into(),
                                                                },
                                                                selected_text_color: Color::BLACK,
                                                                selected_background:
                                                                    iced::Background::Color(
                                                                        theme.palette().primary,
                                                                    ),
                                                                shadow: iced::Shadow::default(),
                                                            }
                                                        })
                                                        .size(mn * 0.02)
                                                    )
                                                    .align_x(iced::Alignment::End)
                                                ],
                                                row![
                                                    container(
                                                        text(self.l10n.get("version"))
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(
                                                        if let Some(ver) = &self.available_update {
                                                            if self.update_in_progress {
                                                                container(
                                                                    text(self.l10n.get("updating"))
                                                                        .size(mn * 0.022),
                                                                )
                                                            } else {
                                                                container(
                                                                    button(
                                                                        text(self.l10n.get_args(
                                                                            "update-to",
                                                                            &[(
                                                                                "ver",
                                                                                ver.as_str(),
                                                                            )],
                                                                        ))
                                                                        .size(mn * 0.015),
                                                                    )
                                                                    .on_press(Message::ApplyUpdate),
                                                                )
                                                            }
                                                        } else {
                                                            container(
                                                                text(format!(
                                                                    "v{}",
                                                                    CURRENT_VERSION
                                                                ))
                                                                .size(mn * 0.022)
                                                                .color(theme.palette().text),
                                                            )
                                                        }
                                                    )
                                                    .align_x(iced::Alignment::End)
                                                ],
                                                column![
                                                    container(row![
                                                        text(self.l10n.get("made-by"))
                                                            .font(SF_PRO_EXPANDED_BOLD)
                                                            .size(mn * 0.015)
                                                            .color(theme.palette().text),
                                                        mouse_area(
                                                            text(" gurbanov")
                                                                .font(SF_PRO_EXPANDED_BOLD)
                                                                .size(mn * 0.015)
                                                                .color(theme.palette().primary)
                                                        )
                                                        .on_press(Message::OpenUrl(String::from(
                                                            "https://github.com/gurbbanov"
                                                        ))),
                                                        text(" ♥")
                                                            .size(mn * 0.015)
                                                            .color(theme.palette().primary)
                                                    ])
                                                    .align_x(Alignment::Center)
                                                    .width(Length::Fill),
                                                    container(
                                                        mouse_area(
                                                            text("☕")
                                                                .size(mn * 0.03)
                                                                .color(theme.palette().primary)
                                                        )
                                                        .on_press(Message::OpenUrl(String::from(
                                                            "https://boosty.to/gurbbanov/donate"
                                                        )))
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(Alignment::Center),
                                                ]
                                                .spacing(s.height * 0.02)
                                            ]
                                            .width(Length::Fill)
                                            .spacing(s.height * 0.03),
                                        )
                                        .direction(
                                            scrollable::Direction::Vertical(
                                                scrollable::Scrollbar::new()
                                                    .width(0)
                                                    .scroller_width(0)
                                            )
                                        )
                                    ]
                                    .width(Length::Fill)
                                    .spacing(s.height * 0.035),
                                )
                                .padding(mn * 0.015)
                                .width(Length::Fixed(mn * 0.7))
                                .height(Length::Fixed(mn * 0.6))
                                .style(move |_| container::Style {
                                    background: Some(iced::Background::Color(Color::from_rgb8(
                                        23, 23, 23,
                                    ))),
                                    border: iced::Border {
                                        radius: (mn * 0.015).into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                            )
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .align_x(iced::Alignment::Center)
                            .align_y(iced::Alignment::Center)
                            .into()
                        }))
                        .on_press(Message::None),
                    )
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center)
                    .style(|_| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgba(
                            0.0, 0.0, 0.0, 0.5,
                        ))),
                        ..Default::default()
                    })
                } else {
                    container(text(""))
                        .width(Length::Fixed(0.0))
                        .height(Length::Fixed(0.0))
                        .into()
                })
                .on_press(Message::None)
                .on_scroll(|_| Message::None)
            ]
            .into()
        } else {
            container(text("window is closed")).into()
        }
    }

    fn page0(&self, size: Size) -> Element<'_, Message> {
        let sh = size.height;
        let sw = size.width / 2.0;

        let slot_size = Size::new(sw, sh);

        let left_items: Vec<Element<'_, Message>> = self
            .page0_left
            .iter()
            .map(|w| {
                container(w.view(
                    &self.time,
                    &self.weather,
                    &self.theme.value(),
                    &self.media_metadata,
                    slot_size,
                    *self.gradient_c1.value(),
                    *self.gradient_c2.value(),
                    self.seek_preview,
                    self.volume_preview,
                    self.volume,
                    self.app_settings.smooth_tick,
                    &self.l10n,
                    &self.app_settings.speed_unit,
                ))
                .width(Length::Fixed(sw))
                .height(Length::Fixed(sh))
                .into()
            })
            .collect();

        let right_items: Vec<Element<'_, Message>> = self
            .page0_right
            .iter()
            .map(|w| {
                container(w.view(
                    &self.time,
                    &self.weather,
                    &self.theme.value(),
                    &self.media_metadata,
                    slot_size,
                    *self.gradient_c1.value(),
                    *self.gradient_c2.value(),
                    self.seek_preview,
                    self.volume_preview,
                    self.volume,
                    self.app_settings.smooth_tick,
                    &self.l10n,
                    &self.app_settings.speed_unit,
                ))
                .width(Length::Fixed(sw))
                .height(Length::Fixed(sh))
                .into()
            })
            .collect();

        let left = vertical_carousel(left_items, sw, sh, self.carousel_page0_left, |index| {
            Message::CarouselChanged(CarouselId::Page0Left, index)
        });

        let right = vertical_carousel(right_items, sw, sh, self.carousel_page0_right, |index| {
            Message::CarouselChanged(CarouselId::Page0Right, index)
        });

        let primary = self.theme.value().palette().primary;

        let t_fullscreen = *self.fullscreen_btn_hover.value();
        let fullscreen_btn_color = Color {
            r: primary.r * t_fullscreen + 0.0 * (1.0 - t_fullscreen),
            g: primary.g * t_fullscreen + 0.0 * (1.0 - t_fullscreen),
            b: primary.b * t_fullscreen + 0.0 * (1.0 - t_fullscreen),
            a: 1.0,
        };

        let t_settings = *self.settings_btn_hover.value();
        let settings_btn_color = Color {
            r: primary.r * t_settings + 0.0 * (1.0 - t_settings),
            g: primary.g * t_settings + 0.0 * (1.0 - t_settings),
            b: primary.b * t_settings + 0.0 * (1.0 - t_settings),
            a: 1.0,
        };

        container(stack![
            row![left, right],
            container(
                mouse_area(
                    button(
                        svg(svg::Handle::from_memory(if self.fullscreen {
                            FULLSCREEN_EXIT_SVG
                        } else {
                            FULLSCREEN_ENTER_SVG
                        }))
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: Some(fullscreen_btn_color),
                            ..Default::default()
                        })
                        .width(Length::Fixed(sw.min(sh) * 0.1 * t_fullscreen.max(0.3)))
                        .height(Length::Fixed(sw.min(sh) * 0.1 * t_fullscreen.max(0.3))),
                    )
                    .style(|_, _| button::Style {
                        background: None,
                        ..Default::default()
                    })
                    .on_press(Message::ToggleFullscreen)
                )
                .on_enter(Message::FullscreenBtnHover(true))
                .on_exit(Message::FullscreenBtnHover(false)),
            )
            .padding(Padding::new(sw.min(sh) * 0.03))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Start),
            container(
                mouse_area(
                    button(
                        svg(svg::Handle::from_memory(include_bytes!(
                            "../icons/settings.svg"
                        )))
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: Some(settings_btn_color),
                            ..Default::default()
                        })
                        .width(Length::Fixed(sw.min(sh) * 0.1 * t_settings.max(0.3)))
                        .height(Length::Fixed(sw.min(sh) * 0.1 * t_settings.max(0.3))),
                    )
                    .style(|_, _| button::Style {
                        background: None,
                        ..Default::default()
                    })
                    .on_press(Message::OpenSettings)
                )
                .on_enter(Message::SettingsBtnHover(true))
                .on_exit(Message::SettingsBtnHover(false)),
            )
            .padding(Padding::new(sw.min(sh) * 0.03))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .align_y(Alignment::Start),
        ])
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .into()
    }

    fn page1(&self, size: Size) -> Element<'_, Message> {
        let items: Vec<Element<'_, Message>> = self
            .page1_widgets
            .iter()
            .map(|w| {
                container(w.view(
                    &self.time,
                    &self.weather,
                    &self.theme.value(),
                    &self.media_metadata,
                    size,
                    *self.gradient_c1.value(),
                    *self.gradient_c2.value(),
                    self.seek_preview,
                    self.volume_preview,
                    self.volume,
                    self.app_settings.smooth_tick,
                    &self.l10n,
                    &self.app_settings.speed_unit,
                ))
                .width(Length::Fixed(size.width))
                .height(Length::Fixed(size.height))
                .into()
            })
            .collect();

        vertical_carousel(
            items,
            size.width,
            size.height,
            self.carousel_page1,
            |index| Message::CarouselChanged(CarouselId::Page1, index),
        )
    }

    fn find_widget_mut(&mut self, id: WidgetId) -> Option<&mut AppWidget> {
        self.page0_left
            .iter_mut()
            .chain(self.page0_right.iter_mut())
            .chain(self.page1_widgets.iter_mut())
            .find(|w| w.id() == id)
    }
}

impl Default for Application {
    fn default() -> Self {
        let app_settings = AppSettings::default();
        let l10n = L10n::new(app_settings.locale.as_str());

        Application {
            time: chrono::Utc::now(),
            weather: WeatherStatus::Loading,
            page0_left: vec![
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L0,
                    ClockStyle::AnalogueHalf(AnalogueClockHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L1,
                    ClockStyle::AnalogueCityHalf(AnalogueClockCityHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L2,
                    ClockStyle::MinimalHalf(MinimalClockHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L3,
                    ClockStyle::MinimalCityHalf(MinimalClockCityHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L4,
                    ClockStyle::AnalogueRectHalf(AnalogueRectClockHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L5,
                    ClockStyle::AnalogueRectCityHalf(AnalogueRectClockCityHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L6,
                    ClockStyle::DigitalHalf(DigitalClockHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L7,
                    ClockStyle::DigitalCityHalf(DigitalClockCityHalf::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_L8,
                    ClockStyle::WorldHalf(WorldClockHalf::default()),
                )),
            ],
            page0_right: vec![
                AppWidget::Media(MediaWidget::default()),
                AppWidget::Calendar(CalendarWidget::new_with_id(
                    WID_R1,
                    CalendarStyle::MonthHalf(MonthCalendarHalf::default()),
                )),
                AppWidget::Calendar(CalendarWidget::new_with_id(
                    WID_R2,
                    CalendarStyle::DateHalf(DateCalendarHalf::default()),
                )),
                AppWidget::Weather(WeatherWidget::new_with_id(
                    WID_R3,
                    WeatherStyle::MinimalHalf(MinimalForecastHalf::default()),
                )),
                AppWidget::Weather(WeatherWidget::new_with_id(
                    WID_R4,
                    WeatherStyle::DetailedHalf(DetailedForecastHalf::default()),
                )),
                AppWidget::Weather(WeatherWidget::new_with_id(
                    WID_R5,
                    WeatherStyle::DailyHalf(DailyForecastHalf::default()),
                )),
            ],
            page1_widgets: vec![
                AppWidget::Media(MediaWidget {
                    id: WID_P0,
                    style: MediaStyle::MediaFull(MediaWidgetFull::default()),
                }),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_P1,
                    ClockStyle::WorldFull(WorldClockFull::default()),
                )),
                AppWidget::Clock(ClockWidget::new_with_id(
                    WID_P2,
                    ClockStyle::AnalogueRectFull(AnalogueRectClockFull::default()),
                )),
            ],
            gradient_c1: Animated::new(
                Color::BLACK,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            gradient_c2: Animated::new(
                Color::BLACK,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            theme: Animated::new(
                Theme::custom(
                    "classic".to_string(),
                    Palette {
                        text: Color::WHITE,
                        primary: color!(169, 169, 169),
                        danger: color!(87, 87, 87),
                        background: color!(0, 0, 0),
                        success: Color::WHITE,
                        warning: color!(240, 157, 10),
                        ..Theme::Moonfly.palette()
                    },
                ),
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            #[cfg(target_os = "windows")]
            playerctl: None,
            #[cfg(target_os = "windows")]
            session: None,
            #[cfg(target_os = "macos")]
            now_playing: None,
            media_metadata: None,
            seek_preview: None,
            volume: 0.3,
            volume_preview: None,
            fullscreen: true,
            fullscreen_btn_hover: Animated::new(
                0.0f32,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            settings_btn_hover: Animated::new(
                0.0f32,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            main_window: None,
            settings_open: false,
            app_settings: app_settings,
            current_page: 0,
            page_width: 800.0,
            drag: DragState::Idle,
            metadata_updating: false,
            l10n: l10n,
            available_update: None,
            update_in_progress: false,
            carousel_page0_left: 0,
            carousel_page0_right: 0,
            carousel_page1: 0,
        }
    }
}
