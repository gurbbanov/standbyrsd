use chrono::prelude::*;
use iced::advanced::{
    Clipboard, Renderer as AdvancedRenderer, Shell,
    layout::{self, Layout},
    renderer,
    widget::{self},
};
use iced::border::Radius;
use iced::font::{Family, Stretch, Style, Weight};
use iced::theme::{Base, Palette};
use iced::time::{self, milliseconds, seconds};
use iced::widget::canvas::{Cache, Frame, LineCap, Path, Stroke, stroke};
use iced::widget::{
    button, canvas, center, column, combo_box, container, responsive, row, stack, svg, text,
};
use iced::window::{self, Id};
use iced::{
    Alignment, Color, Degrees, Element, Font, Length, Padding, Point, Radians, Rectangle, Renderer,
    Settings, Size, Subscription, Task, Theme, Vector, alignment, color, padding,
};
use iced::{Pixels, mouse};
use iced_anim::{Animated, Animation, Easing};
#[cfg(target_os = "macos")]
use media_remote;
use reqwest;
use serde::Deserialize;
use std::cell::Cell;
use std::f64::consts::PI;
use std::time::{Duration, Instant};
use volumecontrol;
#[cfg(target_os = "windows")]
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession, GlobalSystemMediaTransportControlsSessionManager,
};

const SF_PRO_EXPANDED_BOLD: Font = Font {
    family: Family::Name("SF Pro"),
    weight: Weight::Bold,
    stretch: Stretch::Expanded,
    style: Style::Normal,
};

const SF_PRO_COMPRESSED_SEMIBOLD: Font = Font {
    family: Family::Name("SF Pro"),
    weight: Weight::Bold,
    stretch: Stretch::Condensed,
    style: Style::Normal,
};

const SF_PRO_ROUNDED_BLACK: Font = Font {
    family: Family::Name("SF Pro Rounded"),
    weight: Weight::Black,
    ..Font::DEFAULT
};

const SF_PRO_DISPLAY_BOLD: Font = Font {
    family: Family::Name("SF Pro Display"),
    weight: Weight::Bold,
    ..Font::DEFAULT
};

const SF_PRO_DISPLAY_BLACK: Font = Font {
    family: Family::Name("SF Pro Display"),
    weight: Weight::Black,
    ..Font::DEFAULT
};

const FULLSCREEN_EXIT_SVG: &[u8] = include_bytes!("../icons/fullscreen-exit.svg");
const FULLSCREEN_ENTER_SVG: &[u8] = include_bytes!("../icons/fullscreen-enter.svg");

pub fn main() -> iced::Result {
    iced::daemon(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .settings(Settings {
            fonts: vec![
                include_bytes!("../fonts/SF-Pro-Rounded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Expanded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Black.otf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Bold.otf").into(),
                include_bytes!("../fonts/SF-Pro-Compressed.ttf").into(),
            ],
            default_font: SF_PRO_DISPLAY_BOLD,
            ..Settings::default()
        })
        .theme(Application::theme)
        .antialiasing(true)
        .run()
}

const PAGE_COUNT: usize = 2;
const SNAP_THRESHOLD: f32 = 0.025;
const IDLE_MS: u64 = 16;
const SNAP_DURATION_MS: u64 = 420;

#[derive(Debug, Clone)]
enum DragState {
    Idle,
    Active {
        offset_px: f32,
        velocity: f32,
        last_event: Instant,
    },
    Snapping {
        start_offset: f32,
        end_offset: f32,
        velocity: f32,
        started_at: Instant,
    },
}

impl DragState {
    fn is_snapping_done(&self) -> bool {
        if let DragState::Snapping { started_at, .. } = self {
            started_at.elapsed().as_millis() >= SNAP_DURATION_MS as u128
        } else {
            false
        }
    }
}

fn ease_spring(t: f32, v0: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let hermite = 3.0 * t2 - 2.0 * t3;
    let velocity_term = v0 * t * (t - 1.0) * (t - 1.0);
    (hermite + velocity_term).clamp(0.0, 1.0)
}

struct Application {
    time: chrono::DateTime<Local>,
    weather: WeatherStatus,
    page0_left: Vec<AppWidget>,
    page0_right: Vec<AppWidget>,
    page1_widgets: Vec<AppWidget>,
    gradient_c1: Animated<Color>,
    gradient_c2: Animated<Color>,
    theme: Animated<Theme>,
    fullscreen: bool,
    fullscreen_btn_hover: Animated<f32>,
    settings_btn_hover: Animated<f32>,
    #[cfg(target_os = "windows")]
    playerctl: Option<GlobalSystemMediaTransportControlsSessionManager>,
    #[cfg(target_os = "windows")]
    session: Option<GlobalSystemMediaTransportControlsSession>,
    #[cfg(target_os = "macos")]
    now_playing: Option<media_remote::NowPlayingPerl>,
    media_metadata: Option<MediaMetadata>,
    seek_preview: Option<f32>,
    volume: f32,
    volume_preview: Option<f32>,
    main_window: Option<window::Id>,
    settings_open: bool,
    theme_mode_list: combo_box::State<ThemeMode>,
    app_settings: AppSettings,
    current_page: usize,
    page_width: f32,
    drag: DragState,
    metadata_updating: bool,
}

#[derive(Debug, Clone)]
struct AppSettings {
    theme_mode: ThemeMode,
    theme_dark_at: String,
    theme_light_at: String,
    smooth_tick: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::Classic,
            theme_dark_at: "22:00".to_string(),
            theme_light_at: "07:00".to_string(),
            smooth_tick: true,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Tick(chrono::DateTime<Local>),
    FetchWeather,
    WeatherFetched(WeatherStatus),
    OpenMainWindow,
    WindowOpened(Id),
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
    None,
}

impl Application {
    fn new() -> (Self, Task<Message>) {
        (
            Self::default(),
            Task::batch([
                Task::done(Message::OpenMainWindow),
                Task::done(Message::GetPlayer),
                Task::done(Message::FetchWeather),
            ]),
        )
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

    fn theme(&self, _id: Id) -> Theme {
        self.theme.value().clone()
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(local_time) => {
                if local_time != self.time {
                    self.time = local_time;
                }
                Task::none()
            }
            Message::FetchWeather => Task::perform(
                async {
                    let mut w = Weather::default();
                    match w.fetch().await {
                        Ok(()) => WeatherStatus::Ok(w),
                        Err(e) => WeatherStatus::Error(e.to_string()),
                    }
                },
                Message::WeatherFetched,
            ),
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
                                position_origin: chrono::Local::now(),
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
                            position_origin: chrono::Local::now(),
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
                                chrono::Local::now()
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
                                chrono::Local::now()
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
                                chrono::Local::now()
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
            Message::OpenSettings => {
                self.settings_open = true;

                Task::none()
            }
            Message::CloseSettings => {
                self.settings_open = false;

                Task::none()
            }
            Message::None => Task::none(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let clock = time::every(if self.app_settings.smooth_tick {
            milliseconds(16)
        } else {
            seconds(1)
        })
        .map(|_| Message::Tick(chrono::Local::now()));

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

        Subscription::batch([clock, weather, metadata_update, snap_idle, anim, theme])
    }

    fn view(&self, id: Id) -> Element<'_, Message> {
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

                                    slide_pages(
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
                iced::widget::mouse_area(if self.settings_open {
                    container(
                        iced::widget::mouse_area(responsive(move |s| {
                            let mn = s.height.min(s.width);
                            container(
                                container(
                                    column![
                                        row![
                                            container(
                                                text("settings")
                                                    .size(mn * 0.05)
                                                    .color(theme.palette().text)
                                            )
                                            .width(Length::Fill)
                                            .align_x(iced::Alignment::Start),
                                            container(
                                                button(
                                                    container("")
                                                        .width(Length::Fixed(14.0))
                                                        .height(Length::Fixed(14.0))
                                                )
                                                .on_press(Message::CloseSettings)
                                                .width(Length::Fixed(20.0))
                                                .height(Length::Fixed(20.0))
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
                                        column![
                                            row![
                                                container(
                                                    text("theme")
                                                        .size(mn * 0.022)
                                                        .color(theme.palette().text)
                                                )
                                                .width(Length::Fill)
                                                .align_x(iced::Alignment::Start),
                                                container(
                                                    combo_box(
                                                        &self.theme_mode_list,
                                                        "select theme",
                                                        Some(&self.app_settings.theme_mode),
                                                        Message::ThemeModeChanged,
                                                    )
                                                    .width(Length::Fixed(mn * 0.2))
                                                    .input_style(move |_t, _status| {
                                                        iced::widget::text_input::Style {
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
                                                        iced::widget::overlay::menu::Style {
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
                                            if self.app_settings.theme_mode == ThemeMode::AutoCustom
                                            {
                                                row![
                                                    container(row![
                                                        container(
                                                            text("light at")
                                                                .size(mn * 0.022)
                                                                .color(theme.palette().text)
                                                        )
                                                        .width(Length::Fill)
                                                        .align_x(iced::Alignment::Start),
                                                        container(row![
                                                            iced::widget::text_input(
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
                                                                iced::widget::text_input::Style {
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
                                                            iced::widget::text_input(
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
                                                                iced::widget::text_input::Style {
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
                                            if self.app_settings.theme_mode == ThemeMode::AutoCustom
                                            {
                                                row![
                                                    container(
                                                        text("dark at")
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text)
                                                    )
                                                    .width(Length::Fill)
                                                    .align_x(iced::Alignment::Start),
                                                    container(row![
                                                        iced::widget::text_input(
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
                                                            Message::ThemeDarkAtChanged(s, true)
                                                        })
                                                        .width(Length::Fixed(mn * 0.05))
                                                        .style(move |_t, _status| {
                                                            iced::widget::text_input::Style {
                                                                value: theme.palette().text,
                                                                placeholder: theme.palette().text,
                                                                selection: theme.palette().danger,
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
                                                        }),
                                                        text(":")
                                                            .size(mn * 0.022)
                                                            .color(theme.palette().text),
                                                        iced::widget::text_input(
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
                                                            Message::ThemeDarkAtChanged(s, false)
                                                        })
                                                        .width(Length::Fixed(mn * 0.05))
                                                        .style(move |_t, _status| {
                                                            iced::widget::text_input::Style {
                                                                value: theme.palette().text,
                                                                placeholder: theme.palette().text,
                                                                selection: theme.palette().danger,
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
                                                text("smooth tick")
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
                                        ]
                                    ]
                                    .width(Length::Fill)
                                    .spacing(s.height * 0.03),
                                )
                                .padding(mn * 0.015)
                                .width(Length::Fixed(mn * 0.7))
                                .height(Length::Fixed(mn * 0.4))
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
                ))
                .width(Length::Fixed(sw))
                .height(Length::Fixed(sh))
                .into()
            })
            .collect();

        let left = vertical_carousel(left_items, sw, sh);
        let right = vertical_carousel(right_items, sw, sh);

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
                iced::widget::mouse_area(
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
                iced::widget::mouse_area(
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
                ))
                .width(Length::Fixed(size.width))
                .height(Length::Fixed(size.height))
                .into()
            })
            .collect();

        vertical_carousel(items, size.width, size.height)
    }
}

impl Default for Application {
    fn default() -> Self {
        Application {
            time: chrono::Local::now(),
            weather: WeatherStatus::Loading,
            page0_left: vec![
                AppWidget::Clock(ClockWidget::new(ClockStyle::AnalogueHalf(
                    AnalogueClockHalf::default(),
                ))),
                AppWidget::Clock(ClockWidget::new(ClockStyle::MinimalHalf(
                    MinimalClockHalf::default(),
                ))),
                AppWidget::Clock(ClockWidget::new(ClockStyle::AnalogueRectHalf(
                    AnalogueRectClockHalf::default(),
                ))),
                AppWidget::Clock(ClockWidget::new(ClockStyle::DigitalHalf(
                    DigitalClockHalf::default(),
                ))),
            ],
            page0_right: vec![
                AppWidget::Media(MediaWidget {
                    style: MediaStyle::MediaHalf(MediaWidgetHalf::default()),
                }),
                AppWidget::Calendar(CalendarWidget::new(CalendarStyle::MonthHalf(
                    MonthCalendarHalf::default(),
                ))),
                AppWidget::Calendar(CalendarWidget::new(CalendarStyle::DateHalf(
                    DateCalendarHalf::default(),
                ))),
                AppWidget::Weather(WeatherWidget::default()),
                AppWidget::Weather(WeatherWidget::new(WeatherStyle::DetailedHalf(
                    DetailedForecastHalf::default(),
                ))),
                AppWidget::Weather(WeatherWidget::new(WeatherStyle::DailyHalf(
                    DailyForecastHalf::default(),
                ))),
            ],
            page1_widgets: vec![
                AppWidget::Media(MediaWidget {
                    style: MediaStyle::MediaFull(MediaWidgetFull::default()),
                }),
                AppWidget::Clock(ClockWidget::new(ClockStyle::WorldFull(
                    WorldClockFull::default(),
                ))),
                AppWidget::Clock(ClockWidget::new(ClockStyle::AnalogueRectFull(
                    AnalogueRectClockFull::default(),
                ))),
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
            theme_mode_list: combo_box::State::new(vec![
                ThemeMode::Classic,
                ThemeMode::RedDark,
                ThemeMode::AutoSunrise,
                ThemeMode::AutoCustom,
            ]),
            app_settings: AppSettings::default(),
            current_page: 0,
            page_width: 800.0,
            drag: DragState::Idle,
            metadata_updating: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ThemeMode {
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

enum ThemeVariant {
    Classic,
    RedDark,
}

struct SlidePages<'a, M, T, R> {
    offset: f32,
    page_width: f32,
    page_height: f32,
    children: Vec<Element<'a, M, T, R>>,
}

fn slide_pages<'a>(
    offset: f32,
    page_width: f32,
    page_height: f32,
    page0: Element<'a, Message>,
    page1: Element<'a, Message>,
) -> Element<'a, Message> {
    SlidePages {
        offset,
        page_width,
        page_height,
        children: vec![page0, page1],
    }
    .into()
}

impl<'a> From<SlidePages<'a, Message, Theme, Renderer>> for Element<'a, Message, Theme, Renderer> {
    fn from(w: SlidePages<'a, Message, Theme, Renderer>) -> Self {
        Element::new(w)
    }
}

impl<'a> iced::advanced::Widget<Message, Theme, Renderer>
    for SlidePages<'a, Message, Theme, Renderer>
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let pw = self.page_width;
        let ph = self.page_height;
        let child_limits = layout::Limits::new(Size::ZERO, Size::new(pw, ph));

        let children: Vec<layout::Node> = self
            .children
            .iter_mut()
            .enumerate()
            .map(|(i, child)| {
                let mut node =
                    child
                        .as_widget_mut()
                        .layout(&mut tree.children[i], renderer, &child_limits);
                node = node.translate(Vector::new(i as f32 * pw, 0.0));
                node
            })
            .collect();

        layout::Node::with_children(
            limits.resolve(Length::Fill, Length::Fill, Size::new(pw, ph)),
            children,
        )
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.children.iter().map(|c| widget::Tree::new(c)).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.children);
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        let expanded_viewport = Rectangle {
            x: viewport.x - self.page_width,
            y: viewport.y,
            width: viewport.width + self.page_width * 2.0,
            height: viewport.height,
        };

        renderer.with_layer(bounds, |renderer: &mut Renderer| {
            renderer.with_translation(Vector::new(self.offset, 0.0), |renderer: &mut Renderer| {
                for (i, (child, child_layout)) in
                    self.children.iter().zip(layout.children()).enumerate()
                {
                    child.as_widget().draw(
                        &tree.children[i],
                        renderer,
                        theme,
                        style,
                        child_layout,
                        cursor,
                        &expanded_viewport,
                    );
                }
            });
        });
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        shell.publish(Message::UpdatePageWidth(bounds.width));

        if let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            if cursor.is_over(bounds) {
                let dx = match delta {
                    mouse::ScrollDelta::Pixels { x, .. } => *x * 2.0,
                    mouse::ScrollDelta::Lines { x, .. } => *x * 80.0,
                };
                if dx.abs() > 0.3 {
                    shell.publish(Message::DragDelta(dx));
                    return;
                }
            }
        }

        let translated_cursor = match cursor {
            mouse::Cursor::Available(pos) => {
                mouse::Cursor::Available(Point::new(pos.x - self.offset, pos.y))
            }
            other => other,
        };

        for (i, (child, child_layout)) in
            self.children.iter_mut().zip(layout.children()).enumerate()
        {
            child.as_widget_mut().update(
                &mut tree.children[i],
                event,
                child_layout,
                translated_cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let translated_cursor = match cursor {
            mouse::Cursor::Available(pos) => {
                mouse::Cursor::Available(Point::new(pos.x - self.offset, pos.y))
            }
            other => other,
        };

        self.children
            .iter()
            .zip(layout.children())
            .enumerate()
            .map(|(i, (child, child_layout))| {
                child.as_widget().mouse_interaction(
                    &tree.children[i],
                    child_layout,
                    translated_cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Default)]
struct CarouselState {
    current: usize,
    offset_px: f32,
    velocity: f32,
    snap: Option<CarouselSnap>,
    last_event: Option<Instant>,
}

#[derive(Debug, Clone)]
struct CarouselSnap {
    start: f32,
    end: f32,
    velocity: f32,
    started_at: Instant,
}

impl CarouselState {
    fn total_offset(&self, sh: f32) -> f32 {
        if let Some(ref s) = self.snap {
            let elapsed = s.started_at.elapsed().as_secs_f32();
            let t = (elapsed / (SNAP_DURATION_MS as f32 / 1000.0)).min(1.0);
            let dist = s.end - s.start;
            let v0 = if dist.abs() > 0.001 {
                s.velocity / dist
            } else {
                0.0
            };
            s.start + dist * ease_spring(t, v0)
        } else {
            -(self.current as f32) * sh + self.offset_px
        }
    }

    fn is_snap_done(&self) -> bool {
        self.snap.as_ref().map_or(false, |s| {
            s.started_at.elapsed().as_millis() >= SNAP_DURATION_MS as u128
        })
    }

    fn try_snap(&mut self, count: usize, sh: f32) {
        let ratio = self.offset_px / sh;
        let from = self.current;
        let abs_now = -(from as f32) * sh + self.offset_px;

        let (target, abs_end) = if ratio < -SNAP_THRESHOLD && from + 1 < count {
            (from + 1, -((from + 1) as f32) * sh)
        } else if ratio > SNAP_THRESHOLD && from > 0 {
            (from - 1, -((from - 1) as f32) * sh)
        } else {
            (from, -(from as f32) * sh)
        };

        self.current = target;
        self.snap = Some(CarouselSnap {
            start: abs_now,
            end: abs_end,
            velocity: self.velocity,
            started_at: Instant::now(),
        });
        self.offset_px = 0.0;
        self.last_event = None;
    }
}

struct VerticalCarousel<'a> {
    items: Vec<Element<'a, Message>>,
    slot_width: f32,
    slot_height: f32,
}

fn vertical_carousel<'a>(
    items: Vec<Element<'a, Message>>,
    slot_width: f32,
    slot_height: f32,
) -> Element<'a, Message> {
    VerticalCarousel {
        items,
        slot_width,
        slot_height,
    }
    .into()
}

impl<'a> From<VerticalCarousel<'a>> for Element<'a, Message, Theme, Renderer> {
    fn from(w: VerticalCarousel<'a>) -> Self {
        Element::new(w)
    }
}

impl<'a> iced::advanced::Widget<Message, Theme, Renderer> for VerticalCarousel<'a> {
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.slot_width),
            Length::Fixed(self.slot_height),
        )
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<CarouselState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(CarouselState::default())
    }

    fn children(&self) -> Vec<widget::Tree> {
        self.items.iter().map(|c| widget::Tree::new(c)).collect()
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&self.items);
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let sw = self.slot_width;
        let sh = self.slot_height;
        let child_limits = layout::Limits::new(Size::ZERO, Size::new(sw, sh));

        let children: Vec<layout::Node> = self
            .items
            .iter_mut()
            .enumerate()
            .map(|(i, child)| {
                let mut node =
                    child
                        .as_widget_mut()
                        .layout(&mut tree.children[i], renderer, &child_limits);
                node = node.translate(Vector::new(0.0, i as f32 * sh));
                node
            })
            .collect();

        layout::Node::with_children(
            limits.resolve(Length::Fixed(sw), Length::Fixed(sh), Size::new(sw, sh)),
            children,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<CarouselState>();
        let bounds = layout.bounds();
        let sh = self.slot_height;

        let total_offset_y = state.total_offset(sh);
        let total_height = sh * self.items.len() as f32;

        let expanded_viewport = Rectangle {
            x: viewport.x,
            y: viewport.y - total_height,
            width: viewport.width,
            height: viewport.height + total_height * 2.0,
        };

        renderer.with_layer(bounds, |renderer: &mut Renderer| {
            renderer.with_translation(
                Vector::new(0.0, total_offset_y),
                |renderer: &mut Renderer| {
                    for (i, (child, child_layout)) in
                        self.items.iter().zip(layout.children()).enumerate()
                    {
                        child.as_widget().draw(
                            &tree.children[i],
                            renderer,
                            theme,
                            style,
                            child_layout,
                            cursor,
                            &expanded_viewport,
                        );
                    }
                },
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let sh = self.slot_height;
        let count = self.items.len();

        let total_offset_y = {
            let state = tree.state.downcast_mut::<CarouselState>();

            if state.is_snap_done() {
                state.snap = None;
            }

            if state.snap.is_none() {
                if let Some(last) = state.last_event {
                    if last.elapsed().as_millis() >= IDLE_MS as u128 {
                        state.try_snap(count, sh);
                    }
                }
            }

            if let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
                if cursor.is_over(bounds) && state.snap.is_none() {
                    let dy = match delta {
                        mouse::ScrollDelta::Pixels { y, .. } => *y * 2.0,
                        mouse::ScrollDelta::Lines { y, .. } => *y * 80.0,
                    };
                    if dy.abs() > 0.3 {
                        let max_drag = if state.current > 0 { sh } else { 0.0 };
                        let min_drag = if state.current + 1 < count { -sh } else { 0.0 };
                        state.offset_px = (state.offset_px + dy).clamp(min_drag, max_drag);
                        state.velocity = dy;
                        state.last_event = Some(Instant::now());

                        if dy.abs() < 1.5 {
                            state.try_snap(count, sh);
                        }

                        return;
                    }
                }
            }

            state.total_offset(sh)
        };

        let translated_cursor = match cursor {
            mouse::Cursor::Available(point) => {
                mouse::Cursor::Available(point - Vector::new(0.0, total_offset_y))
            }
            other => other,
        };

        for (i, (child, child_layout)) in self.items.iter_mut().zip(layout.children()).enumerate() {
            child.as_widget_mut().update(
                &mut tree.children[i],
                event,
                child_layout,
                translated_cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<CarouselState>();
        let total_offset_y = state.total_offset(self.slot_height);

        let translated_cursor = match cursor {
            mouse::Cursor::Available(point) => {
                mouse::Cursor::Available(point - Vector::new(0.0, total_offset_y))
            }
            other => other,
        };

        self.items
            .iter()
            .zip(layout.children())
            .enumerate()
            .map(|(i, (child, child_layout))| {
                child.as_widget().mouse_interaction(
                    &tree.children[i],
                    child_layout,
                    translated_cursor,
                    viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }
}

enum AppWidget {
    Calendar(CalendarWidget),
    Clock(ClockWidget),
    Weather(WeatherWidget),
    Media(MediaWidget),
}

impl AppWidget {
    pub fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
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
    ) -> Element<'a, Message> {
        match self {
            AppWidget::Clock(w) => w.view(time, weather, theme, size, smooth_tick),
            AppWidget::Calendar(w) => w.view(time),
            AppWidget::Weather(w) => w.view(theme, time, weather, size),
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
}

trait ClearCache {
    fn clear_cache(&self);
}

struct CalendarWidget {
    style: CalendarStyle,
}

impl CalendarWidget {
    fn new(style: CalendarStyle) -> Self {
        Self { style }
    }

    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        self.style.view(time)
    }
}

impl ClearCache for CalendarWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

enum CalendarStyle {
    MonthHalf(MonthCalendarHalf),
    DateHalf(DateCalendarHalf),
}

impl CalendarStyle {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        match self {
            CalendarStyle::MonthHalf(c) => c.view(time),
            CalendarStyle::DateHalf(c) => c.view(time),
        }
    }
}

impl ClearCache for CalendarStyle {
    fn clear_cache(&self) {
        match self {
            CalendarStyle::MonthHalf(c) => c.cache.clear(),
            CalendarStyle::DateHalf(c) => c.cache.clear(),
        }
    }
}

#[derive(Default)]
struct MonthCalendarHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl MonthCalendarHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a MonthCalendarHalf, &'a DateTime<Local>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, now) = self;
        let palette = theme.palette();

        let layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let w = frame.width() * 0.95;
            let h = frame.height();

            let first_day_of_month = weekday_to_number(
                &NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                    .unwrap()
                    .weekday(),
            );

            let last_day_of_month = NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
                .unwrap_or_else(|| NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap())
                .pred_opt()
                .unwrap()
                .day() as usize;

            let num_rows =
                ((first_day_of_month - 1 + last_day_of_month) as f32 / 7.0).ceil() as usize;

            let columns = 7usize;

            let cell_w_by_width = w / columns as f32;
            let total_rows = (num_rows + 2) as f32;
            let cell_w_by_height = h / total_rows;
            let cell_w = cell_w_by_width.min(cell_w_by_height);

            let cell_h = cell_w;
            let font_size = cell_w * 0.38;
            let month_font_size = cell_w * 0.6;

            let grid_w = cell_w * columns as f32;
            let total_h = month_font_size + cell_h * (1.0 + num_rows as f32);
            let offset_x = (w - grid_w) * 0.5;
            let offset_y = (h - total_h) * 0.5;

            frame.fill_text(canvas::Text {
                content: format!("   {}", now.format("%B")).to_uppercase(),
                position: Point::new(offset_x, offset_y + month_font_size * 0.5),
                size: month_font_size.into(),
                color: color!(255, 0, 0),
                font: SF_PRO_DISPLAY_BLACK,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Center,
                ..canvas::Text::default()
            });

            let weekdays = ["M", "T", "W", "T", "F", "S", "S"];

            for (col, label) in weekdays.iter().enumerate() {
                let x = offset_x + col as f32 * cell_w + cell_w * 0.5;
                let y = offset_y + month_font_size + cell_h * 0.5;
                let is_weekend = col >= 5;
                frame.fill_text(canvas::Text {
                    content: label.to_string(),
                    position: Point::new(x, y),
                    size: font_size.into(),
                    color: if is_weekend {
                        palette.danger
                    } else {
                        palette.text
                    },
                    font: SF_PRO_DISPLAY_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }

            let mut slot = first_day_of_month - 1;

            for day in 1..=last_day_of_month {
                let col = slot % 7;
                let row = slot / 7;

                let x = offset_x + col as f32 * cell_w + cell_w * 0.5;
                let y = offset_y + month_font_size + cell_h + row as f32 * cell_h + cell_h * 0.5;

                let is_today = day == now.day() as usize;
                let is_weekend = col >= 5;

                if is_today {
                    let r = cell_w * 0.5;
                    frame.fill(&Path::circle(Point::new(x, y), r), color!(255, 0, 0));
                }

                frame.fill_text(canvas::Text {
                    content: day.to_string(),
                    position: Point::new(x, y),
                    size: font_size.into(),
                    color: if is_today {
                        palette.success
                    } else if is_weekend {
                        palette.danger
                    } else {
                        palette.text
                    },
                    font: SF_PRO_DISPLAY_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..canvas::Text::default()
                });

                slot += 1;
            }
        });

        vec![layer]
    }
}

#[derive(Default)]
struct DateCalendarHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl DateCalendarHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a DateCalendarHalf, &'a DateTime<Local>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, time) = self;
        let palette = theme.palette();
        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            frame.with_save(|frame| {
                let size = frame.width().min(frame.height());
                let center = Point::new(frame.width() / 2.0, frame.height() / 2.0);

                frame.fill_text(canvas::Text {
                    content: format!("{:3}", time.weekday()),
                    size: Pixels(size * 0.2),
                    position: Point::new(center.x - size * 0.02, center.y - size * 0.25),
                    color: color!(255, 0, 0),
                    align_y: alignment::Vertical::Bottom,
                    align_x: text::Alignment::Right,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: time.format("%b").to_string(),
                    size: Pixels(size * 0.2),
                    position: Point::new(center.x + size * 0.02, center.y - size * 0.25),
                    color: palette.danger,
                    align_y: alignment::Vertical::Bottom,
                    align_x: text::Alignment::Left,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: format!("{}", time.day()),
                    size: Pixels(size * 0.8),
                    position: Point::new(center.x, center.y + size * 0.05),
                    color: palette.text,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Center,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });
            });
        });
        vec![dynamic_layer]
    }
}

struct ClockWidget {
    style: ClockStyle,
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self {
            style: ClockStyle::AnalogueHalf(AnalogueClockHalf::default()),
        }
    }
}

impl ClockWidget {
    fn new(style: ClockStyle) -> Self {
        Self { style }
    }

    fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        size: Size,
        smooth_tick: bool,
    ) -> Element<'a, Message> {
        self.style.view(time, weather, theme, size, smooth_tick)
    }
}

impl ClearCache for ClockWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

enum ClockStyle {
    DigitalHalf(DigitalClockHalf),
    AnalogueHalf(AnalogueClockHalf),
    MinimalHalf(MinimalClockHalf),
    AnalogueRectHalf(AnalogueRectClockHalf),
    AnalogueRectFull(AnalogueRectClockFull),
    WorldFull(WorldClockFull),
}

impl ClockStyle {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        size: Size,
        smooth_tick: bool,
    ) -> Element<'a, Message> {
        match self {
            ClockStyle::DigitalHalf(clock) => clock.view(time),
            ClockStyle::AnalogueHalf(clock) => clock.view(time, smooth_tick),
            ClockStyle::MinimalHalf(clock) => clock.view(time, smooth_tick),
            ClockStyle::AnalogueRectHalf(clock) => clock.view(time, smooth_tick),
            ClockStyle::AnalogueRectFull(clock) => clock.view(time, smooth_tick),
            ClockStyle::WorldFull(clock) => clock.view(time, weather, theme, size),
        }
    }
}

impl ClearCache for ClockStyle {
    fn clear_cache(&self) {
        match self {
            ClockStyle::AnalogueHalf(clock) => clock.clear_cache(),
            ClockStyle::MinimalHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueRectHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueRectFull(clock) => clock.clear_cache(),
            ClockStyle::WorldFull(clock) => clock.clear_cache(),
            _ => {}
        }
    }
}

#[derive(Default)]
struct DigitalClockHalf {
    cache: Cache,
}

impl DigitalClockHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        self.cache.clear();
        canvas((self, time))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a DigitalClockHalf, &'a DateTime<Local>) {
    type State = ();
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, now) = self;

        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let palette = theme.palette();
            let s = frame.width().min(frame.height()) * 0.9;
            let center = frame.center();

            let pad = s * 0.04;
            let line_len = pad * 0.9;
            let line_width = s * 0.011;
            let half_w = line_width / 2.0;
            let radius = line_width / 2.0;
            let avail = s - 2.0 * pad;
            let step = avail / 14.0;

            let subsec = now.nanosecond() as f32 / 1_000_000_000.0;
            let current_sec = now.second() as i32;
            const TAIL: i32 = 60;

            frame.with_save(|frame| {
                frame.translate(Vector::new(center.x, center.y));

                for i in 0..60i32 {
                    let dist = (current_sec - i).rem_euclid(60);
                    let alpha = if dist == 0 {
                        subsec
                    } else if dist < TAIL {
                        let d = dist as f32 - (1.0 - subsec);
                        (1.0 - d / TAIL as f32).max(0.0)
                    } else if dist == TAIL {
                        let d = TAIL as f32 - (1.0 - subsec);
                        (1.0 - d / TAIL as f32).max(0.0)
                    } else {
                        0.15
                    };

                    let color = Color {
                        a: alpha,
                        ..palette.text
                    };

                    let offset = 7;
                    let pos = (i + offset).rem_euclid(60);

                    let side = (pos / 15) as usize;
                    let j = (pos % 15) as f32;
                    let t = -s / 2.0 + pad + j * step;

                    let (px, py) = match side {
                        0 => (t, -s / 2.0),
                        1 => (s / 2.0, t),
                        2 => (-t, s / 2.0),
                        3 => (-s / 2.0, -t),
                        _ => unreachable!(),
                    };

                    let angle = py.atan2(px) + std::f32::consts::FRAC_PI_2;

                    let perp_angle = match side {
                        0 | 2 => px.abs().atan2((s / 2.0).abs()),
                        1 | 3 => py.abs().atan2((s / 2.0).abs()),
                        _ => unreachable!(),
                    };
                    let adjusted_len = line_len / perp_angle.cos().max(0.1);

                    frame.with_save(|frame| {
                        frame.translate(Vector::new(px, py));
                        frame.rotate(Radians(angle));
                        frame.fill(
                            &Path::rounded_rectangle(
                                Point::new(-half_w, 0.0),
                                Size::new(line_width, adjusted_len),
                                Radius::new(radius),
                            ),
                            color,
                        );
                    });
                }
            });

            let font_size = s * 0.47;

            // часы
            frame.fill_text(canvas::Text {
                content: format!("{:02}", now.hour()),
                position: Point {
                    x: center.x - font_size * 0.2,
                    y: center.y,
                },
                size: font_size.into(),
                color: palette.text,
                font: SF_PRO_COMPRESSED_SEMIBOLD,
                align_x: text::Alignment::Right,
                align_y: alignment::Vertical::Center,
                ..Default::default()
            });

            // двоеточие мигающее
            let colon = if now.second() % 2 == 0 { ":" } else { " " };
            frame.fill_text(canvas::Text {
                content: colon.to_string(),
                position: Point {
                    x: center.x,
                    y: center.y - font_size * 0.1,
                },
                size: (font_size * 1.1).into(),
                color: palette.danger,
                font: SF_PRO_COMPRESSED_SEMIBOLD,
                align_x: text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..Default::default()
            });

            // минуты
            frame.fill_text(canvas::Text {
                content: format!("{:02}", now.minute()),
                position: Point {
                    x: center.x + font_size * 0.2,
                    y: center.y,
                },
                size: font_size.into(),
                color: palette.text,
                font: SF_PRO_COMPRESSED_SEMIBOLD,
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Center,
                ..Default::default()
            });
        });

        vec![dynamic_layer]
    }
}

#[derive(Default)]
struct AnalogueClockHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueHalf,
}

impl AnalogueClockHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>, smooth_tick: bool) -> Element<'a, Message> {
        stack![self.clock_frame.view(), self.hands.view(time, smooth_tick)].into()
    }
}

impl ClearCache for AnalogueClockHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
struct Hands {
    cache: Cache,
}

impl Hands {
    fn view<'a>(&'a self, time: &'a DateTime<Local>, smooth_tick: bool) -> Element<'a, Message> {
        self.cache.clear();

        canvas((self, time, smooth_tick))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a Hands, &'a DateTime<Local>, bool) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();
        let (widget, now, smooth_tick) = self;

        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            let radius = frame.width().min(frame.height()) / 2.3;
            let seconds = if *smooth_tick {
                now.second() as f32 + now.nanosecond() as f32 / 1_000_000_000.0
            } else {
                now.second() as f32
            };
            let minutes_portion = Radians::from(hand_rotation(now.minute(), 60)) / 12.0;
            let hour_hand_angle = Radians::from(hand_rotation(now.hour(), 12)) + minutes_portion;
            let minute_angle = hand_rotation(now.minute() * 15 + now.second() / 4, 900);
            let second_angle = hand_rotation_sec(seconds, 60.0);

            frame.translate(Vector::new(center.x, center.y));

            // hours
            let hour_circle_r = radius * 0.03;
            let hour_neck_len = radius * 0.12;
            let hour_body_len = radius * 0.55;
            let hour_neck_width = radius / 30.0;
            let hour_body_width = radius / 15.0;
            let hour_circle = Path::circle(Point::ORIGIN, hour_circle_r);

            let hour_neck = Path::new(|p| {
                p.move_to(Point::new(0.0, -hour_circle_r));
                p.line_to(Point::new(0.0, -(hour_circle_r + hour_neck_len)));
            });

            let hour_body = Path::new(|p| {
                p.move_to(Point::new(0.0, -(hour_circle_r + hour_neck_len)));
                p.line_to(Point::new(0.0, -hour_body_len));
            });

            frame.with_save(|frame| {
                frame.rotate(hour_hand_angle);
                frame.with_save(|f| {
                    f.translate(Vector::new(2.0, 2.0));
                    let shadow = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.6,
                    };

                    f.stroke(
                        &hour_neck,
                        Stroke {
                            width: hour_neck_width * 1.5,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );

                    f.stroke(
                        &hour_body,
                        Stroke {
                            width: hour_body_width * 1.5,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );
                });

                frame.stroke(
                    &hour_circle,
                    Stroke {
                        width: hour_neck_width,
                        style: stroke::Style::Solid(palette.text),
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &hour_neck,
                    Stroke {
                        width: hour_neck_width,
                        style: stroke::Style::Solid(palette.text),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &hour_body,
                    Stroke {
                        width: hour_body_width,
                        style: stroke::Style::Solid(palette.text),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );
            });

            // minutes
            let min_circle_r = radius * 0.03;
            let min_neck_len = radius * 0.12;
            let min_body_len = radius * 0.95;
            let min_neck_width = radius / 30.0;
            let min_body_width = radius / 15.0;

            let min_circle = Path::circle(Point::ORIGIN, min_circle_r);

            let min_neck = Path::new(|p| {
                p.move_to(Point::new(0.0, -min_circle_r));
                p.line_to(Point::new(0.0, -(min_circle_r + min_neck_len)));
            });

            let min_body = Path::new(|p| {
                p.move_to(Point::new(0.0, -(min_circle_r + min_neck_len)));
                p.line_to(Point::new(0.0, -min_body_len));
            });

            frame.with_save(|frame| {
                frame.rotate(minute_angle);

                frame.with_save(|f| {
                    f.translate(Vector::new(0.5, 0.5));

                    let shadow = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.4,
                    };

                    f.stroke(
                        &min_neck,
                        Stroke {
                            width: min_neck_width * 1.5,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );

                    f.stroke(
                        &min_body,
                        Stroke {
                            width: min_body_width * 1.5,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );
                });

                frame.stroke(
                    &min_circle,
                    Stroke {
                        width: min_neck_width,
                        style: stroke::Style::Solid(palette.text),
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &min_neck,
                    Stroke {
                        width: min_neck_width,
                        style: stroke::Style::Solid(palette.text),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &min_body,
                    Stroke {
                        width: min_body_width,
                        style: stroke::Style::Solid(palette.text),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );
            });

            // seconds
            let sec_tail_len = radius * 0.16;
            let sec_line_len = radius;
            let sec_circle_r = radius * 0.02;
            let sec_width = radius / 80.0;

            let sec_tail = Path::new(|p| {
                p.move_to(Point::new(0.0, sec_tail_len));
                p.line_to(Point::new(0.0, sec_circle_r));
            });

            let sec_line = Path::new(|p| {
                p.move_to(Point::new(0.0, -sec_circle_r));
                p.line_to(Point::new(0.0, -sec_line_len));
            });

            let sec_circle = Path::circle(Point::ORIGIN, sec_circle_r);

            frame.with_save(|frame| {
                frame.rotate(second_angle);

                frame.with_save(|f| {
                    f.translate(Vector::new(1.5, 1.5));
                    let shadow = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.25,
                    };
                    let shadow_stroke = Stroke {
                        width: sec_width,
                        style: stroke::Style::Solid(shadow),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    };
                    f.stroke(&sec_tail, shadow_stroke.clone());
                    f.stroke(&sec_line, shadow_stroke);
                });

                let sec_stroke = Stroke {
                    width: sec_width,
                    style: stroke::Style::Solid(palette.warning),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                };

                frame.stroke(&sec_tail, sec_stroke.clone());
                frame.stroke(&sec_line, sec_stroke);

                frame.stroke(
                    &sec_circle,
                    Stroke {
                        width: sec_width,
                        style: stroke::Style::Solid(palette.warning),
                        ..Stroke::default()
                    },
                );
            });
        });

        vec![dynamic_layer]
    }
}

#[derive(Default)]
struct ClockFrameAnalogueHalf {
    cache: Cache,
}

impl ClockFrameAnalogueHalf {
    fn view(&self) -> Element<'_, Message> {
        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for ClockFrameAnalogueHalf {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();

        let static_layer = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();

            frame.translate(Vector::new(center.x, center.y));

            let radius = frame.width().min(frame.height()) / 2.3;

            for hour in 1..=12 {
                let angle = Radians::from(hand_rotation(hour, 12)) - Radians::from(Degrees(90.0));

                let x = radius * angle.0.cos();
                let y = radius * angle.0.sin();

                frame.fill_text(canvas::Text {
                    content: format!("{hour}"),
                    size: (radius / 4.5).into(),
                    position: Point::new(x * 0.75, y * 0.75),
                    color: palette.text,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: SF_PRO_ROUNDED_BLACK,
                    ..canvas::Text::default()
                });
            }

            let mut color;

            for tick in 0..60 {
                let angle = hand_rotation(tick, 60);
                let width = if tick % 5 == 0 {
                    color = palette.primary;
                    radius * 0.016
                } else {
                    color = palette.danger;
                    radius * 0.016
                };
                frame.with_save(|frame| {
                    frame.rotate(angle);
                    frame.fill(
                        &Path::rounded_rectangle(
                            Point::new(-width / 2.0, radius - width * 6.0),
                            Size::new(width, width * 6.0),
                            Radius::new(width / 2.0),
                        ),
                        color,
                    );
                });
            }
        });

        vec![static_layer]
    }
}

#[derive(Default)]
struct MinimalClockHalf {
    hands: Hands,
    clock_frame: ClockFrameMinimalHalf,
}

impl MinimalClockHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>, smooth_tick: bool) -> Element<'a, Message> {
        stack![self.clock_frame.view(), self.hands.view(time, smooth_tick)].into()
    }
}

impl ClearCache for MinimalClockHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
struct ClockFrameMinimalHalf {
    cache: Cache,
}

impl ClockFrameMinimalHalf {
    fn view(&self) -> Element<'_, Message> {
        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for ClockFrameMinimalHalf {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();

        let static_layer = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();

            frame.translate(Vector::new(center.x, center.y));

            let radius = frame.width().min(frame.height()) / 2.9;

            for hour in 1..=12 {
                let angle = Radians::from(hand_rotation(hour, 12)) - Radians::from(Degrees(90.0));

                let width = radius * 0.055;

                frame.with_save(|frame| {
                    frame.rotate(angle);
                    frame.fill(
                        &Path::rounded_rectangle(
                            Point::new(0.0, radius),
                            Size::new(width, width * 5.0),
                            Radius::new(width / 2.0),
                        ),
                        palette.text,
                    );
                });
            }
        });

        vec![static_layer]
    }
}

#[derive(Default)]
struct AnalogueRectClockHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueRectHalf,
}

impl AnalogueRectClockHalf {
    fn view<'a>(&'a self, time: &'a DateTime<Local>, smooth_tick: bool) -> Element<'a, Message> {
        stack![self.clock_frame.view(), self.hands.view(time, smooth_tick)].into()
    }
}

impl ClearCache for AnalogueRectClockHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
struct ClockFrameAnalogueRectHalf {
    cache: Cache,
}

impl ClockFrameAnalogueRectHalf {
    fn view(&self) -> Element<'_, Message> {
        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for ClockFrameAnalogueRectHalf {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();

        let static_layer = self.cache.draw(renderer, bounds.size(), |frame| {
            let size = frame.width().min(frame.height());
            let scale = size / 960.0;

            let offset_x = (frame.width() - size) / 2.0;
            let offset_y = (frame.height() - size) / 2.0;

            let padding = scale * 70.0;
            let inner_padding_hour = scale * 130.0;
            let inner_padding_min = scale * 30.0;

            let top_left = Point::new(offset_x + padding, offset_y + padding);
            let bottom_left = Point::new(offset_x + padding, offset_y + size - padding);
            let width = size - padding * 2.0;
            let height = size - padding * 2.0;
            let center = Point::new(offset_x + size / 2.0, offset_y + size / 2.0);

            let doli_minutes = vec![
                0.0612, 0.1378, 0.2755, 0.3367, 0.3929, 0.4439, 0.5561, 0.6071, 0.6633, 0.7245,
                0.8622, 0.9388,
            ];
            let doli_hours = vec![0.2092, 0.5, 0.7908];

            frame.with_save(|frame| {
                // upper side
                for i in &doli_minutes {
                    let point = Point::new(top_left.x + width * i, top_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x + inner_padding_min * (dx / dy),
                        point.y + inner_padding_min,
                    );

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(top_left.x + width * i, top_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x, point.y + inner_padding_min * 2.0)
                    } else {
                        Point::new(
                            point.x + inner_padding_hour * (dx / dy),
                            point.y + inner_padding_hour,
                        )
                    };

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                // bottom side
                for i in &doli_minutes {
                    let point = Point::new(bottom_left.x + width * i, bottom_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x - inner_padding_min * (dx / dy),
                        point.y - inner_padding_min,
                    );

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(bottom_left.x + width * i, bottom_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x, point.y - inner_padding_min * 2.0)
                    } else {
                        Point::new(
                            point.x - inner_padding_hour * (dx / dy),
                            point.y - inner_padding_hour,
                        )
                    };

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                // left side
                for i in &doli_minutes {
                    let point = Point::new(top_left.x, top_left.y + height * i);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x + inner_padding_min,
                        point.y + inner_padding_min * (dy / dx),
                    );

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(top_left.x, top_left.y + height * i);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x + inner_padding_min * 2.0, point.y)
                    } else {
                        Point::new(
                            point.x + inner_padding_hour,
                            point.y + inner_padding_hour * (dy / dx),
                        )
                    };

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                // right side
                for i in &doli_minutes {
                    let point = Point::new(top_left.x + width, top_left.y + height * i);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x - inner_padding_min,
                        point.y - inner_padding_min * (dy / dx),
                    );

                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(top_left.x + width, top_left.y + height * i);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x - inner_padding_min * 2.0, point.y)
                    } else {
                        Point::new(
                            point.x - inner_padding_hour,
                            point.y - inner_padding_hour * (dy / dx),
                        )
                    };
                    frame.stroke(
                        &Path::line(point, end_point),
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(6.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                let hours = vec![
                    (
                        "12",
                        Point::new(center.x, offset_y + inner_padding_hour * 1.6),
                    ),
                    (
                        "3",
                        Point::new(offset_x + size - inner_padding_hour * 1.6, center.y),
                    ),
                    (
                        "6",
                        Point::new(center.x, offset_y + size - inner_padding_hour * 1.6),
                    ),
                    (
                        "9",
                        Point::new(offset_x + inner_padding_hour * 1.6, center.y),
                    ),
                ];

                for (hour, point) in hours {
                    frame.fill_text(canvas::Text {
                        content: hour.to_string(),
                        size: Pixels(125.0 * scale),
                        position: point,
                        color: palette.text,
                        align_x: text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });
                }
            })
        });
        vec![static_layer]
    }
}

#[derive(Default)]
struct AnalogueRectClockFull {
    hands: Hands,
    clock_frame: ClockFrameAnalogueRectFull,
}

impl AnalogueRectClockFull {
    fn view<'a>(&'a self, time: &'a DateTime<Local>, smooth_tick: bool) -> Element<'a, Message> {
        stack![
            self.clock_frame.view(time),
            self.hands.view(time, smooth_tick)
        ]
        .into()
    }
}

impl ClearCache for AnalogueRectClockFull {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
struct ClockFrameAnalogueRectFull {
    last_day: Cell<u32>,
    cache: Cache,
}

impl ClockFrameAnalogueRectFull {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a ClockFrameAnalogueRectFull, &'a DateTime<Local>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, time) = self;
        let palette = theme.palette();

        let static_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);

            let padding = scale * 70.0;
            let inner_padding_hourtb = scale * 250.0; //inner padding for hours located at top and bottom
            let inner_padding_hourlr = scale * 130.0; //inner padding for hours located at left and right
            let inner_padding_min = scale * 120.0;

            let top_left = Point::new(padding, padding);
            let top_right = Point::new(frame.width() - padding, padding);
            let bottom_right = Point::new(frame.width() - padding, frame.height() - padding);
            let bottom_left = Point::new(padding, frame.height() - padding);

            let center = frame.center();

            let doli_minutes = vec![
                0.1739, 0.2363, 0.2854, 0.3270, 0.3913, 0.4197, 0.4461, 0.4707, 0.5293, 0.5539,
                0.5803, 0.6087, 0.6730, 0.7146, 0.7637, 0.8261,
            ];

            let doli_hours = vec![0.0907, 0.3611, 0.5, 0.6411, 0.9093];

            let width = frame.width() - padding * 2.0;
            let height = frame.height();

            frame.with_save(|frame| {
                //upper side
                for i in &doli_minutes {
                    let point = Point::new(top_left.x + width * i, top_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x + (inner_padding_min - point.y) * (dx / dy),
                        inner_padding_min,
                    );

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(4.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(top_left.x + width * i, top_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x, inner_padding_min)
                    } else {
                        Point::new(
                            point.x + (inner_padding_hourtb - point.y) * (dx / dy),
                            inner_padding_hourtb,
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(10.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                //bottom side
                for i in &doli_minutes {
                    let point = Point::new(bottom_left.x + width * i, bottom_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = Point::new(
                        point.x + ((frame.height() - inner_padding_min) - point.y) * (dx / dy),
                        frame.height() - inner_padding_min,
                    );

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.danger)
                            .with_width(4.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                for i in &doli_hours {
                    let point = Point::new(bottom_left.x + width * i, bottom_left.y);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if *i == 0.5 {
                        Point::new(point.x, frame.height() - inner_padding_min)
                    } else {
                        Point::new(
                            point.x
                                + ((frame.height() - inner_padding_hourtb) - point.y) * (dx / dy),
                            frame.height() - inner_padding_hourtb,
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.primary)
                            .with_width(10.0 * scale)
                            .with_line_cap(LineCap::Round),
                    );
                }

                //left side
                for i in 1..10 {
                    let point = Point::new(top_left.x, height * 0.1 * i as f32);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if i == 5 {
                        Point::new(point.x + inner_padding_hourlr * 1.5, point.y)
                    } else {
                        Point::new(
                            inner_padding_hourlr,
                            point.y + (inner_padding_hourlr - point.x) * (dy / dx),
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(if i == 5 {
                                palette.primary
                            } else {
                                palette.danger
                            })
                            .with_width(if i == 5 { 10.0 * scale } else { 4.0 * scale })
                            .with_line_cap(LineCap::Round),
                    );
                }

                //right side
                for i in 1..10 {
                    let point = Point::new(top_left.x + width, height * 0.1 * i as f32);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if i == 5 {
                        Point::new(point.x - inner_padding_hourlr * 1.5, point.y)
                    } else {
                        Point::new(
                            frame.width() - inner_padding_hourlr,
                            point.y
                                + ((frame.width() - inner_padding_hourlr) - point.x) * (dy / dx),
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(if i == 5 {
                                palette.primary
                            } else {
                                palette.danger
                            })
                            .with_width(if i == 5 { 10.0 * scale } else { 4.0 * scale })
                            .with_line_cap(LineCap::Round),
                    );
                }

                frame.fill_text(canvas::Text {
                    content: time.weekday().to_string().to_uppercase(),
                    size: Pixels(50.0 * scale),
                    position: Point::new(frame.width() * 2.0 / 3.0, frame.center().y),
                    color: color!(255, 0, 0),
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: SF_PRO_EXPANDED_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: time.day().to_string(),
                    size: Pixels(50.0 * scale),
                    position: Point::new(
                        frame.width() * 2.0 / 3.0 + 110.0 * scale,
                        frame.center().y,
                    ),
                    color: palette.text,
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: SF_PRO_EXPANDED_BOLD,
                    ..canvas::Text::default()
                });

                let hours = vec![
                    ("12", Point::new(frame.center().x, 210.0 * scale)),
                    (
                        "3",
                        Point::new(frame.width() - 360.0 * scale, frame.center().y),
                    ),
                    (
                        "6",
                        Point::new(frame.center().x, frame.height() - 210.0 * scale),
                    ),
                    ("9", Point::new(360.0 * scale, frame.center().y)),
                ];

                for (hour, point) in hours {
                    frame.fill_text(canvas::Text {
                        content: format!("{hour}"),
                        size: Pixels(125.0 * scale),
                        position: point,
                        color: palette.text,
                        align_x: text::Alignment::Center,
                        align_y: alignment::Vertical::Center,
                        font: SF_PRO_EXPANDED_BOLD,
                        ..canvas::Text::default()
                    });
                }
            })
        });

        vec![static_layer]
    }
}

#[derive(Default)]
struct WorldClockFull {
    minute: Cell<u32>,
    cache: Cache,
}

impl WorldClockFull {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        size: Size,
    ) -> Element<'a, Message> {
        if time.minute() != self.minute.get() {
            self.minute.set(time.minute());
            self.cache.clear();
        }

        let map = svg(svg::Handle::from_memory(include_bytes!(
            "../icons/world_map.svg"
        )))
        .style(move |_theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().primary),
        })
        .height(Length::Fill)
        .width(size.width * 0.85);

        stack![
            container(map)
                .padding(Padding {
                    top: 0.0,
                    bottom: 0.0,
                    right: size.width * 0.015,
                    left: 0.0
                })
                .align_right(size.width)
                .width(Length::Fill)
                .height(Length::Fill),
            canvas((self, time, weather))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a WorldClockFull, &'a DateTime<Local>, &'a WeatherStatus) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, time, weather) = self;
        let palette = theme.palette();

        let static_layer = match weather {
            WeatherStatus::Ok(w) => widget.cache.draw(renderer, bounds.size(), |frame| {
                let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);

                frame.with_save(|frame| {
                    let city = w.city.as_ref().unwrap();
                    let (lat, lon) = w.coordinate.as_ref().unwrap();

                    let map_width = frame.width() * 0.85;
                    let map_height = map_width * (921.0 / 2146.0);

                    let map_offset_y = (frame.height() - map_height) / 2.0;

                    let point = lat_lon_to_xy(
                        lat.parse::<f64>().unwrap(),
                        lon.parse::<f64>().unwrap(),
                        map_width,
                        map_height,
                    ) + Vector::new(frame.width() * 0.15, map_offset_y);

                    let dot_size = map_width * 0.015;
                    let dot_outer = Path::circle(point, dot_size);
                    let dot_inner = Path::circle(point, dot_size * 0.7);

                    frame.fill(&dot_outer, palette.text);
                    frame.fill(&dot_inner, palette.warning);

                    frame.fill_text(canvas::Text {
                        content: format!("{}", city),
                        size: Pixels(50.0 * scale),
                        position: Point::new(
                            frame.center().x - (bounds.width * 0.45),
                            frame.center().y + (bounds.height * 0.12),
                        ),
                        color: palette.warning,
                        align_y: alignment::Vertical::Center,
                        align_x: text::Alignment::Left,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}:{:02}", time.hour(), time.minute()),
                        size: Pixels(200.0 * scale),
                        position: Point::new(
                            frame.center().x - (bounds.width * 0.45),
                            frame.center().y + (bounds.height * 0.25),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Center,
                        align_x: text::Alignment::Left,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });
                });
            }),
            WeatherStatus::Error(e) => widget.cache.draw(renderer, bounds.size(), |frame| {
                let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);
                frame.fill_text(canvas::Text {
                    content: String::from("Location unavailable"),
                    size: Pixels(50.0 * scale),
                    position: Point::new(
                        frame.center().x - (bounds.width * 0.45),
                        frame.center().y + (bounds.height * 0.2),
                    ),
                    color: palette.warning,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Left,
                    font: SF_PRO_DISPLAY_BLACK,
                    ..canvas::Text::default()
                });
            }),
            _ => widget.cache.draw(renderer, bounds.size(), |frame| {
                let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);
                frame.fill_text(canvas::Text {
                    content: String::from("Unknown"),
                    size: Pixels(50.0 * scale),
                    position: Point::new(
                        frame.center().x - (bounds.width * 0.45),
                        frame.center().y + (bounds.height * 0.2),
                    ),
                    color: palette.warning,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Left,
                    font: SF_PRO_DISPLAY_BLACK,
                    ..canvas::Text::default()
                });
            }),
        };

        let night_layer = {
            let mut frame = Frame::new(renderer, bounds.size());

            if let WeatherStatus::Ok(_) = weather {
                let map_width = frame.width() * 0.85;
                let map_height = map_width * (921.0 / 2146.0);
                let map_offset_y = (frame.height() - map_height) / 2.0;

                let map_bounds = Rectangle {
                    x: frame.width() * 0.15,
                    y: map_offset_y,
                    width: map_width,
                    height: map_height,
                };

                let to_canvas = |lat: f64, lon: f64| -> Point {
                    let x = map_bounds.x + ((lon + 180.0) / 360.0) as f32 * map_bounds.width
                        - (map_width.min(map_height) * 0.0401);
                    let y = map_bounds.y + ((90.0 - lat) / 180.0) as f32 * map_bounds.height;
                    Point::new(x, y)
                };

                let (sub_lat, sub_lon) = subsolar_point();

                for (lat, lon) in MAP_DOTS {
                    let lat_r = lat.to_radians();
                    let lon_r = lon.to_radians();

                    let nx = lat_r.cos() * lon_r.cos();
                    let ny = lat_r.cos() * lon_r.sin();
                    let nz = lat_r.sin();

                    let sun = (
                        sub_lat.to_radians().cos() * sub_lon.to_radians().cos(),
                        sub_lat.to_radians().cos() * sub_lon.to_radians().sin(),
                        sub_lat.to_radians().sin(),
                    );

                    let dot = nx * sun.0 + ny * sun.1 + nz * sun.2;

                    let color = if dot < 0.0 {
                        Color::from_rgb(
                            0x51 as f32 / 255.0,
                            0x51 as f32 / 255.0,
                            0x51 as f32 / 255.0,
                        )
                    } else {
                        continue;
                    };

                    let p = to_canvas(*lat, *lon);

                    let circle = Path::circle(p, bounds.height.min(bounds.width) * 0.009);
                    frame.fill(&circle, color);
                }

                draw_night_overlay(&mut frame, map_bounds, sub_lat, sub_lon);
            }

            frame.into_geometry()
        };

        vec![static_layer, night_layer]
    }
}

impl ClearCache for WorldClockFull {
    fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
struct Weather {
    city: Option<String>,
    coordinate: Option<(String, String)>,
    current: Option<CurrentForecast>,
    daily: Option<DailyForecast>,
}

impl Weather {
    async fn fetch(&mut self) -> Result<(), reqwest::Error> {
        let ip = reqwest::get("https://api.ipify.org").await?.text().await?;

        let info = geolocation::find(&ip).unwrap();

        let response: Weather = reqwest::get(
            format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=precipitation_probability_max,apparent_temperature_max,apparent_temperature_min,weather_code,uv_index_max,sunset,sunrise,daylight_duration&current=temperature_2m,is_day,wind_speed_10m,precipitation,weather_code,apparent_temperature&past_days=0&forecast_days=7&timezone=auto", info.latitude, info.longitude),
        )
        .await?
        .json::<Self>()
        .await?;

        *self = Weather {
            city: Some(info.city.replace("\"", "")),
            coordinate: Some((info.latitude, info.longitude)),
            ..response
        };

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct CurrentForecast {
    interval: i32,
    is_day: u8,
    precipitation: f32,
    temperature_2m: f32,
    wind_speed_10m: f32,
    weather_code: u8,
    apparent_temperature: f32,
}

#[derive(Clone, Debug, Deserialize)]
struct DailyForecast {
    apparent_temperature_max: Vec<f32>,
    apparent_temperature_min: Vec<f32>,
    precipitation_probability_max: Vec<f32>,
    weather_code: Vec<u8>,
    uv_index_max: Vec<f32>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

struct WeatherWidget {
    style: WeatherStyle,
}

impl Default for WeatherWidget {
    fn default() -> Self {
        Self {
            style: WeatherStyle::MinimalHalf(MinimalForecastHalf::default()),
        }
    }
}

impl WeatherWidget {
    fn new(style: WeatherStyle) -> Self {
        Self { style }
    }

    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        self.style.view(time, theme, weather, size)
    }
}

impl ClearCache for WeatherWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

#[derive(Clone, Debug, Default)]
enum WeatherStatus {
    #[default]
    Loading,
    Ok(Weather),
    Error(String),
}

enum WeatherStyle {
    MinimalHalf(MinimalForecastHalf),
    DetailedHalf(DetailedForecastHalf),
    DailyHalf(DailyForecastHalf),
}

impl WeatherStyle {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        match self {
            Self::MinimalHalf(w) => w.view(theme, weather, size),
            Self::DetailedHalf(w) => w.view(theme, weather, size),
            Self::DailyHalf(w) => w.view(theme, time, weather, size),
        }
    }
}

impl ClearCache for WeatherStyle {
    fn clear_cache(&self) {
        match self {
            Self::MinimalHalf(w) => w.cache.clear(),
            Self::DetailedHalf(w) => w.cache.clear(),
            Self::DailyHalf(w) => w.cache.clear(),
        }
    }
}

#[derive(Default)]
struct MinimalForecastHalf {
    cache: Cache,
}

impl MinimalForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        let w = size.width;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 100.0 * scale;
        let icon_x = w * 0.05;
        let icon_y = h / 2.0 + 200.0 * scale - icon_size - 20.0 * scale;

        let icon: Element<Message> = match weather {
            WeatherStatus::Ok(w_data) => {
                let code = w_data.current.as_ref().unwrap().weather_code;
                if (code == 0 || code == 1) && w_data.current.as_ref().unwrap().is_day == 0 {
                    svg(svg::Handle::from_memory(wmo_code_svg(100)))
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: if theme.name() == "red_dark" {
                                Some(theme.palette().primary)
                            } else {
                                None
                            },
                        })
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                } else {
                    svg(svg::Handle::from_memory(wmo_code_svg(code)))
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: if theme.name() == "red_dark" {
                                Some(theme.palette().primary)
                            } else {
                                None
                            },
                        })
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                }
            }
            // WeatherStatus::Error(e) => button("Retry").on_press(Message::FetchWeather).into(),
            _ => svg(svg::Handle::from_memory(wmo_code_svg(255)))
                .style(move |_theme: &Theme, _status| svg::Style {
                    color: Some(theme.palette().primary),
                })
                .width(Length::Fixed(icon_size))
                .height(Length::Fixed(icon_size))
                .into(),
        };

        stack![
            canvas((self, weather))
                .width(Length::Fill)
                .height(Length::Fill),
            container(icon)
                .padding(padding::top(icon_y).left(icon_x))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a MinimalForecastHalf, &'a WeatherStatus) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();
        let (widget, weather) = self;

        let static_layer = match weather {
            WeatherStatus::Ok(w) => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.with_save(|frame| {
                    let city = w.city.as_ref().unwrap();
                    let current = w.current.as_ref().unwrap();
                    let daily = w.daily.as_ref().unwrap();

                    let w = frame.width();
                    let h = frame.height();

                    let scale = w / 960.0;

                    frame.fill_text(canvas::Text {
                        content: format!("{}", city),
                        size: Pixels(w.min(h) * 0.1),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 330.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.37),
                        position: Point::new(w * 0.05, frame.center().y + 50.0 * scale.min(h / 1080.0)),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}", wmo_code_description(current.weather_code)),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 340.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!(
                            "H:{:.0}° L:{:.0}°",
                            daily.apparent_temperature_max[0], daily.apparent_temperature_min[0]
                        ),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 420.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });
                })
            }),
            // WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {
            //     frame.fill_text(canvas::Text {
            //         content: String::from("Weather\nis loading"),
            //         size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
            //         position: frame.center(),
            //         color: palette.text,
            //         align_y: alignment::Vertical::Center,
            //         align_x: text::Alignment::Center,
            //         font: SF_PRO_DISPLAY_BOLD,
            //         ..canvas::Text::default()
            //     });
            //     widget.cache.clear();
            // }),
            _/*WeatherStatus::Error(e)*/ => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.fill_text(canvas::Text {
                    content: String::from("Weather\nUnavailable"),
                    size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
                    position: frame.center(),
                    color: palette.text,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Center,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });
            }),
        };
        vec![static_layer]
    }
}

#[derive(Default)]
struct DetailedForecastHalf {
    cache: Cache,
}

impl DetailedForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        let w = size.width;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 80.0 * scale;
        let icon_x = w * 0.83;
        let icon_y = h / 2.0 - 330.0 * scale - icon_size - 20.0 * scale;

        let icon = match weather {
            WeatherStatus::Ok(w_data) => {
                let code = w_data.current.as_ref().unwrap().weather_code;
                if (code == 0 || code == 1) && w_data.current.as_ref().unwrap().is_day == 0 {
                    svg(svg::Handle::from_memory(wmo_code_svg(100)))
                        .style(move |theme: &Theme, _status| svg::Style {
                            color: if theme.name() == "red_dark" {
                                Some(theme.palette().primary)
                            } else {
                                None
                            },
                        })
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                } else {
                    svg(svg::Handle::from_memory(wmo_code_svg(code)))
                        .style(move |theme: &Theme, _status| svg::Style {
                            color: if theme.name() == "red_dark" {
                                Some(theme.palette().primary)
                            } else {
                                None
                            },
                        })
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                }
            }
            _ => svg(svg::Handle::from_memory(wmo_code_svg(255)))
                .style(move |_theme: &Theme, _status| svg::Style {
                    color: Some(theme.palette().primary),
                })
                .width(Length::Fixed(icon_size))
                .height(Length::Fixed(icon_size)),
        };

        stack![
            canvas((self, weather))
                .width(Length::Fill)
                .height(Length::Fill),
            container(icon)
                .padding(padding::top(icon_y).left(icon_x))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a DetailedForecastHalf, &'a WeatherStatus) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, weather) = self;
        let palette = theme.palette();

        let static_layer = match weather {
            WeatherStatus::Ok(w) => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.with_save(|frame| {
                    let city = w.city.as_ref().unwrap();
                    let current = w.current.as_ref().unwrap();
                    let daily = w.daily.as_ref().unwrap();

                    let w = frame.width();
                    let h = frame.height();

                    let scale = w / 960.0;

                    frame.fill_text(canvas::Text {
                        content: format!("{}", city),
                        size: Pixels(w.min(h) * 0.1),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 330.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.2),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 130.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↑{:.0}°", daily.apparent_temperature_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 250.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↓{:.0}°", daily.apparent_temperature_min[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 150.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Precip"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 30.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: daily
                            .precipitation_probability_max
                            .iter()
                            .enumerate()
                            .find(|(_, num)| **num >= 30.0)
                            .map_or("None for 7d".to_string(), |(i, &v)| {
                                format!("{} % in {}d", v, i)
                            }),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 30.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Wind"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 130.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{} m/s", current.wind_speed_10m),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 130.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("UVI"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 280.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}", daily.uv_index_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 280.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Feels Like"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 430.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.apparent_temperature),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 430.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });
                });
            }),
            // WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {
            //     frame.fill_text(canvas::Text {
            //         content: String::from("Weather\nis loading"),
            //         size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
            //         position: frame.center(),
            //         color: palette.text,
            //         align_y: alignment::Vertical::Center,
            //         align_x: text::Alignment::Center,
            //         font: SF_PRO_DISPLAY_BOLD,
            //         ..canvas::Text::default()
            //     });
            //     widget.cache.clear();
            // }),
            _/*WeatherStatus::Error(e)*/ => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.fill_text(canvas::Text {
                    content: String::from("Weather\nUnavailable"),
                    size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
                    position: frame.center(),
                    color: palette.text,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Center,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });
            }),
        };
        vec![static_layer]
    }
}

#[derive(Default)]
struct DailyForecastHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl DailyForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        let w = size.width;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 80.0 * scale;
        let icon_x = w * 0.83;
        let icon_y = h / 2.0 - 330.0 * scale - icon_size - 20.0 * scale;

        let (icon, daily_icons): (Element<Message>, Vec<Element<Message>>) = match weather {
            WeatherStatus::Ok(w_data) => {
                let current = w_data.current.as_ref().unwrap();
                let code = if (current.weather_code == 0 || current.weather_code == 1)
                    && current.is_day == 0
                {
                    100u8
                } else {
                    current.weather_code
                };

                let current_icon = svg(svg::Handle::from_memory(wmo_code_svg(code)))
                    .style(move |_theme: &Theme, _status| svg::Style {
                        color: if theme.name() == "red_dark" {
                            Some(theme.palette().primary)
                        } else {
                            None
                        },
                    })
                    .width(Length::Fixed(icon_size))
                    .height(Length::Fixed(icon_size))
                    .into();

                let daily = w_data.daily.as_ref();

                let icons = match daily {
                    Some(d) => (1..=4)
                        .filter_map(|i| d.weather_code.get(i).copied())
                        .map(|code| {
                            svg(svg::Handle::from_memory(wmo_code_svg(code)))
                                .style(move |_theme: &Theme, _status| svg::Style {
                                    color: if theme.name() == "red_dark" {
                                        Some(theme.palette().primary)
                                    } else {
                                        None
                                    },
                                })
                                .width(Length::Fixed(icon_size * 1.3))
                                .height(Length::Fixed(icon_size * 1.3))
                                .into()
                        })
                        .collect(),
                    None => vec![],
                };

                (current_icon, icons)
            }
            _ => (
                svg(svg::Handle::from_memory(wmo_code_svg(255)))
                    .width(Length::Fixed(icon_size))
                    .height(Length::Fixed(icon_size))
                    .into(),
                vec![],
            ),
        };

        let daily_column = column(daily_icons).spacing(45.0 * scale);

        stack![
            canvas((self, time, weather))
                .width(Length::Fill)
                .height(Length::Fill),
            container(icon)
                .padding(padding::top(icon_y).left(icon_x))
                .width(Length::Fill)
                .height(Length::Fill),
            container(daily_column)
                .padding(padding::top(h / 2.0 - 130.0 * scale).left(w * 0.3))
                .width(Length::Fill)
                .height(Length::Fill)
        ]
        .into()
    }
}

impl<'a> canvas::Program<Message>
    for (
        &'a DailyForecastHalf,
        &'a DateTime<Local>,
        &'a WeatherStatus,
    )
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, time, weather) = self;
        let palette = theme.palette();

        let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let today = weekday_to_number(&time.weekday());

        let mut curr_padding = -50.0;
        let mut counter = 1;

        let static_layer = match weather {
            WeatherStatus::Ok(w) => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.with_save(|frame| {
                    let city = w.city.as_ref().unwrap();
                    let current = w.current.as_ref().unwrap();
                    let daily = w.daily.as_ref().unwrap();

                    let w = frame.width();
                    let h = frame.height();

                    let scale = w / 960.0;

                    frame.fill_text(canvas::Text {
                        content: format!("{}", city),
                        size: Pixels(w.min(h) * 0.1),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 330.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.2),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 130.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↑{:.0}°", daily.apparent_temperature_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 250.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↓{:.0}°", daily.apparent_temperature_min[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 150.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.danger,
                        align_y: alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    for weekday in today..7 {
                        frame.fill_text(canvas::Text {
                            content: format!("{}", weekdays[weekday]),
                            size: Pixels(w.min(h) * 0.08),
                            position: Point::new(
                                w * 0.05,
                                frame.center().y + curr_padding * scale.min(h / 1080.0),
                            ),
                            color: palette.text,
                            align_y: alignment::Vertical::Bottom,
                            font: SF_PRO_DISPLAY_BOLD,
                            ..canvas::Text::default()
                        });

                        frame.fill_text(canvas::Text {
                            content: format!("{:.0}°", daily.apparent_temperature_min[counter]),
                            size: Pixels(w.min(h) * 0.08),
                            position: Point::new(
                                w * 0.80,
                                frame.center().y + curr_padding * scale.min(h / 1080.0),
                            ),
                            color: palette.danger,
                            align_y: alignment::Vertical::Bottom,
                            align_x: text::Alignment::Right,
                            font: SF_PRO_DISPLAY_BOLD,
                            ..canvas::Text::default()
                        });

                        frame.fill_text(canvas::Text {
                            content: format!("{:.0}°", daily.apparent_temperature_max[counter]),
                            size: Pixels(w.min(h) * 0.08),
                            position: Point::new(
                                w * 0.95,
                                frame.center().y + curr_padding * scale.min(h / 1080.0),
                            ),
                            color: palette.text,
                            align_y: alignment::Vertical::Bottom,
                            align_x: text::Alignment::Right,
                            font: SF_PRO_DISPLAY_BOLD,
                            ..canvas::Text::default()
                        });

                        curr_padding += 150.0;
                        counter += 1;
                        if counter == 5 {
                            break;
                        }
                    }

                    if counter != 5 {
                        for weekday in 0..(5 as i32 - counter as i32).abs() as usize {
                            frame.fill_text(canvas::Text {
                                content: format!("{}", weekdays[weekday]),
                                size: Pixels(w.min(h) * 0.08),
                                position: Point::new(
                                    w * 0.05,
                                    frame.center().y + curr_padding * scale.min(h / 1080.0),
                                ),
                                color: palette.text,
                                align_y: alignment::Vertical::Bottom,
                                font: SF_PRO_DISPLAY_BOLD,
                                ..canvas::Text::default()
                            });

                            frame.fill_text(canvas::Text {
                                content: format!("{:.0}°", daily.apparent_temperature_min[counter]),
                                size: Pixels(w.min(h) * 0.08),
                                position: Point::new(
                                    w * 0.80,
                                    frame.center().y + curr_padding * scale.min(h / 1080.0),
                                ),
                                color: palette.danger,
                                align_y: alignment::Vertical::Bottom,
                                align_x: text::Alignment::Right,
                                font: SF_PRO_DISPLAY_BOLD,
                                ..canvas::Text::default()
                            });

                            frame.fill_text(canvas::Text {
                                content: format!("{:.0}°", daily.apparent_temperature_max[counter]),
                                size: Pixels(w.min(h) * 0.08),
                                position: Point::new(
                                    w * 0.95,
                                    frame.center().y + curr_padding * scale.min(h / 1080.0),
                                ),
                                color: palette.text,
                                align_y: alignment::Vertical::Bottom,
                                align_x: text::Alignment::Right,
                                font: SF_PRO_DISPLAY_BOLD,
                                ..canvas::Text::default()
                            });

                            curr_padding += 150.0;
                            counter += 1;
                        }
                    }
                });
            }),
            // WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {
            //     frame.fill_text(canvas::Text {
            //         content: String::from("Weather\nis loading"),
            //         size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
            //         position: frame.center(),
            //         color: palette.text,
            //         align_y: alignment::Vertical::Center,
            //         align_x: text::Alignment::Center,
            //         font: SF_PRO_DISPLAY_BOLD,
            //         ..canvas::Text::default()
            //     });
            //     widget.cache.clear();
            // }),
            _/*WeatherStatus::Error(e)*/ => widget.cache.draw(renderer, bounds.size(), |frame| {
                frame.fill_text(canvas::Text {
                    content: String::from("Weather\nUnavailable"),
                    size: Pixels((frame.width() / 2.0).min(frame.height()) * 0.2),
                    position: frame.center(),
                    color: palette.text,
                    align_y: alignment::Vertical::Center,
                    align_x: text::Alignment::Center,
                    font: SF_PRO_DISPLAY_BOLD,
                    ..canvas::Text::default()
                });
            }),
        };
        vec![static_layer]
    }
}

struct MediaWidget {
    style: MediaStyle,
}

impl MediaWidget {
    fn view<'a>(
        &'a self,
        media_metadata: &'a Option<MediaMetadata>,
        theme: &'a Theme,
        size: Size,
        gc1: Color,
        gc2: Color,
        time: &'a DateTime<Local>,
        seek_preview: Option<f32>,
        voluem_preview: Option<f32>,
        volume: f32,
    ) -> Element<'a, Message> {
        self.style.view(
            media_metadata,
            theme,
            size,
            gc1,
            gc2,
            time,
            seek_preview,
            voluem_preview,
            volume,
        )
    }
}

impl ClearCache for MediaWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

enum MediaStyle {
    MediaHalf(MediaWidgetHalf),
    MediaFull(MediaWidgetFull),
}

impl MediaStyle {
    fn view<'a>(
        &'a self,
        media_metadata: &'a Option<MediaMetadata>,
        theme: &'a Theme,
        size: Size,
        gc1: Color,
        gc2: Color,
        time: &'a DateTime<Local>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
    ) -> Element<'a, Message> {
        match self {
            MediaStyle::MediaHalf(m) => m.view(
                media_metadata,
                theme,
                size,
                time,
                seek_preview,
                volume_preview,
                volume,
            ),
            MediaStyle::MediaFull(m) => m.view(
                media_metadata,
                theme,
                size,
                gc1,
                gc2,
                time,
                seek_preview,
                volume_preview,
                volume,
            ),
            _ => unimplemented!(),
        }
    }
}

impl ClearCache for MediaStyle {
    fn clear_cache(&self) {
        match self {
            MediaStyle::MediaHalf(m) => m.cache.clear(),
            MediaStyle::MediaFull(m) => m.cache.clear(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Default)]
struct MediaWidgetHalf {
    cache: Cache,
}

impl MediaWidgetHalf {
    fn view<'a>(
        &'a self,
        media_metadata: &'a Option<MediaMetadata>,
        theme: &'a Theme,
        size: Size,
        time: &'a DateTime<Local>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
    ) -> Element<'a, Message> {
        let s = size.width.min(size.height);
        let palette = theme.palette();

        let thumbnail = if let Some(handle) =
            media_metadata.as_ref().and_then(|m| m.thumbnail.as_ref())
        {
            container(
                iced::widget::image(handle.clone())
                    .width(Length::Fixed(s * 0.35))
                    .height(Length::Fixed(s * 0.35))
                    .content_fit(iced::ContentFit::ScaleDown),
            )
            .width(Length::Fixed(s * 0.35))
            .height(Length::Fixed(s * 0.35))
        } else {
            let handle = svg::Handle::from_memory(include_bytes!("../icons/media-thumbnail.svg"));

            container(svg(handle).width(Length::Fixed(s)).height(Length::Fixed(s)))
                .width(Length::Fixed(s * 0.35))
                .height(Length::Fixed(s * 0.35))
        };

        let vol = volume_preview.unwrap_or(volume);

        let vol_icon = if vol == 0.0 {
            include_bytes!("../icons/silent.svg").as_ref()
        } else if vol < 0.33 {
            include_bytes!("../icons/low-volume.svg").as_ref()
        } else if vol < 0.66 {
            include_bytes!("../icons/med-volume.svg").as_ref()
        } else {
            include_bytes!("../icons/full-volume.svg").as_ref()
        };

        let icon_size = s * 0.1;
        let bar_w = s * 0.04;
        let bar_h = s * 0.25;

        let volume_control = column![
            svg(svg::Handle::from_memory(vol_icon))
                .style(move |_: &Theme, _| svg::Style {
                    color: Some(palette.text),
                    ..Default::default()
                })
                .width(Length::Fixed(icon_size))
                .height(Length::Fixed(icon_size)),
            canvas(VolumeBar {
                progress: vol,
                preview: volume_preview,
                bg_color: palette.danger,
                bar_color: palette.text,
                radius: bar_w * 0.5,
                orientation: Orientation::Vertical,
            })
            .width(Length::Fixed(bar_w))
            .height(Length::Fixed(bar_h)),
        ]
        .spacing(s * 0.02)
        .align_x(iced::Alignment::Center);

        let top_row = row![
            thumbnail,
            iced::widget::Space::new().width(Length::Fill),
            volume_control,
        ]
        .width(Length::Fixed(s * 0.8))
        .align_y(iced::Alignment::Start);

        let (title, artist, is_playing, position, duration, position_ms, duration_ms) =
            match media_metadata {
                Some(m) => (
                    m.title.clone(),
                    m.artist.clone(),
                    m.is_playing,
                    if m.is_playing {
                        let elapsed = (*time - m.position_origin).num_milliseconds();
                        ((m.position / 10000000) * 1000 + elapsed) / 1000
                    } else {
                        m.position / 10000000
                    },
                    m.duration / 10000000,
                    if m.is_playing {
                        (m.position / 10000) + (*time - m.position_origin).num_milliseconds()
                    } else {
                        m.position / 10000
                    },
                    m.duration / 10000,
                ),
                None => (
                    "Not playing".to_string(),
                    "—".to_string(),
                    false,
                    0,
                    0,
                    0,
                    0,
                ),
            };

        let btn = |handle: svg::Handle, size: f32, msg: Message| -> Element<Message> {
            container(
                button(
                    svg(handle)
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: Some(palette.text),
                            ..Default::default()
                        })
                        .width(Length::Fixed(size))
                        .height(Length::Fixed(size)),
                )
                .on_press(msg)
                .style(|_, _| button::Style {
                    background: None,
                    ..Default::default()
                }),
            )
            .width(Length::Fixed(size))
            .center_x(size)
            .into()
        };

        let fmt_time = |secs: i64| format!("{:02}:{:02}", secs / 60, secs % 60);

        let timecode = row![
            text(fmt_time(position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
            iced::widget::Space::new().width(Length::Fill),
            text(fmt_time(duration))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
        ]
        .width(Length::Fixed(s * 0.8));

        let controls = row![
            btn(
                svg::Handle::from_memory(include_bytes!("../icons/previous.svg")),
                s * 0.12,
                Message::PreviousTrack
            ),
            if is_playing {
                btn(
                    svg::Handle::from_memory(include_bytes!("../icons/pause.svg")),
                    s * 0.12,
                    Message::Pause,
                )
            } else {
                btn(
                    svg::Handle::from_memory(include_bytes!("../icons/play.svg")),
                    s * 0.12,
                    Message::Play,
                )
            },
            btn(
                svg::Handle::from_memory(include_bytes!("../icons/next.svg")),
                s * 0.12,
                Message::NextTrack
            ),
        ]
        .spacing(s * 0.15)
        .align_y(iced::Alignment::Center);

        let content = column![
            top_row,
            column![
                text(title)
                    .size(s * 0.05)
                    .font(SF_PRO_DISPLAY_BOLD)
                    .width(Length::Fixed(s * 0.4))
                    .shaping(iced::widget::text::Shaping::Advanced)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .color(palette.text),
                text(artist)
                    .size(s * 0.03)
                    .width(Length::Fixed(s * 0.4))
                    .shaping(iced::widget::text::Shaping::Advanced)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .font(SF_PRO_DISPLAY_BOLD)
                    .color(palette.primary),
            ]
            .spacing(s * 0.02),
            canvas(SeekBar {
                progress: if duration_ms > 0 {
                    (position_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
                } else {
                    0.0
                },
                preview: seek_preview,
                bg_color: palette.danger,
                bar_color: palette.text,
                radius: s * 0.05,
            })
            .width(Length::Fixed(s * 0.8))
            .height(Length::Fixed(s * 0.03)),
            timecode,
            container(controls)
                .width(Length::Fixed(s * 0.8))
                .align_x(iced::Alignment::Center),
        ]
        .spacing(s * 0.04)
        .align_x(iced::Alignment::Start);

        container(
            container(content)
                .width(Length::Fixed(s))
                .height(Length::Fixed(s))
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::Alignment::Center)
        .align_y(iced::Alignment::Center)
        .into()
    }
}

#[derive(Default)]
struct MediaWidgetFull {
    cache: Cache,
}

impl MediaWidgetFull {
    fn view<'a>(
        &'a self,
        media_metadata: &'a Option<MediaMetadata>,
        theme: &'a Theme,
        size: Size,
        gc1: Color,
        gc2: Color,
        time: &'a DateTime<Local>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
    ) -> Element<'a, Message> {
        let s = size.height.min(size.width / 2.0);
        let palette = theme.palette();

        let thumbnail = if let Some(handle) =
            media_metadata.as_ref().and_then(|m| m.thumbnail.as_ref())
        {
            container(
                iced::widget::image(handle.clone())
                    .width(Length::Fixed(s))
                    .height(Length::Fixed(s))
                    .content_fit(iced::ContentFit::Contain),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center)
        } else {
            let handle = svg::Handle::from_memory(include_bytes!("../icons/media-thumbnail.svg"));

            container(svg(handle).width(Length::Fixed(s)).height(Length::Fixed(s)))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::Alignment::Center)
                .align_y(iced::Alignment::Center)
        };

        let (title, artist, is_playing, position, duration, position_ms, duration_ms) =
            match media_metadata {
                Some(m) => (
                    m.title.clone(),
                    m.artist.clone(),
                    m.is_playing,
                    if m.is_playing {
                        let elapsed = (*time - m.position_origin).num_seconds();
                        ((m.position / 10000000) + elapsed).max(0)
                    } else {
                        m.position / 10000000
                    },
                    m.duration / 10000000,
                    if m.is_playing {
                        (m.position / 10000) + (*time - m.position_origin).num_milliseconds()
                    } else {
                        m.position / 10000
                    },
                    m.duration / 10000,
                ),
                None => (
                    "Not playing".to_string(),
                    "—".to_string(),
                    false,
                    0,
                    0,
                    0,
                    0,
                ),
            };

        let btn = |handle: svg::Handle, size: f32, msg: Message| -> Element<Message> {
            container(
                button(
                    svg(handle)
                        .style(move |_theme: &Theme, _status| svg::Style {
                            color: Some(palette.text),
                            ..Default::default()
                        })
                        .width(Length::Fixed(size))
                        .height(Length::Fixed(size)),
                )
                .on_press(msg)
                .style(|_, _| button::Style {
                    background: None,
                    ..Default::default()
                }),
            )
            .width(Length::Fixed(size))
            .center_x(size)
            .into()
        };

        let fmt_time = |secs: i64| format!("{:02}:{:02}", secs / 60, secs % 60);

        let timecode = row![
            text(fmt_time(position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
            iced::widget::Space::new().width(Length::Fill),
            text(fmt_time(duration))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
        ]
        .width(Length::Fixed(s * 0.8));

        let controls = row![
            btn(
                svg::Handle::from_memory(include_bytes!("../icons/previous.svg")),
                s * 0.18,
                Message::PreviousTrack
            ),
            if is_playing {
                btn(
                    svg::Handle::from_memory(include_bytes!("../icons/pause.svg")),
                    s * 0.18,
                    Message::Pause,
                )
            } else {
                btn(
                    svg::Handle::from_memory(include_bytes!("../icons/play.svg")),
                    s * 0.18,
                    Message::Play,
                )
            },
            btn(
                svg::Handle::from_memory(include_bytes!("../icons/next.svg")),
                s * 0.18,
                Message::NextTrack
            ),
        ]
        .spacing(s * 0.12)
        .align_y(iced::Alignment::Center);

        let vol = volume_preview.unwrap_or(volume);

        let vol_icon = if vol == 0.0 {
            include_bytes!("../icons/silent.svg").as_ref()
        } else if vol < 0.33 {
            include_bytes!("../icons/low-volume.svg").as_ref()
        } else if vol < 0.66 {
            include_bytes!("../icons/med-volume.svg").as_ref()
        } else {
            include_bytes!("../icons/full-volume.svg").as_ref()
        };

        let icon_size = s * 0.06;
        let bar_w = s * 0.7;
        let bar_h = s * 0.03;

        let volume_control = row![
            canvas(VolumeBar {
                progress: vol,
                preview: volume_preview,
                bg_color: palette.danger,
                bar_color: palette.text,
                radius: bar_w * 0.5,
                orientation: Orientation::Horizontal,
            })
            .width(Length::Fixed(bar_w))
            .height(Length::Fixed(bar_h)),
            svg(svg::Handle::from_memory(vol_icon))
                .style(move |_: &Theme, _| svg::Style {
                    color: Some(palette.text),
                    ..Default::default()
                })
                .width(Length::Fixed(icon_size))
                .height(Length::Fixed(icon_size)),
        ]
        .spacing(s * 0.02)
        .align_y(iced::Alignment::Center);

        let content = column![
            column![
                volume_control,
                column![
                    text(title)
                        .size(s * 0.09)
                        .font(SF_PRO_DISPLAY_BOLD)
                        .color(palette.text)
                        .width(Length::Fixed(s * 0.8))
                        .shaping(iced::widget::text::Shaping::Advanced)
                        .wrapping(iced::widget::text::Wrapping::None),
                    text(artist)
                        .size(s * 0.05)
                        .font(SF_PRO_DISPLAY_BOLD)
                        .color(palette.primary)
                        .width(Length::Fixed(s * 0.8))
                        .shaping(iced::widget::text::Shaping::Advanced)
                        .wrapping(iced::widget::text::Wrapping::None),
                ]
                .spacing(s * 0.008)
            ]
            .align_x(iced::Alignment::Start)
            .spacing(s * 0.02),
            container(controls)
                .width(Length::Fixed(s * 0.8))
                .align_x(iced::Alignment::Center),
            column![
                canvas(SeekBar {
                    progress: if duration_ms > 0 {
                        (position_ms as f32 / duration_ms as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                    preview: seek_preview,
                    bg_color: palette.danger,
                    bar_color: palette.text,
                    radius: s * 0.07,
                })
                .height(Length::Fixed(s * 0.04))
                .width(Length::Fixed(s * 0.8)),
                timecode
            ]
            .spacing(s * 0.02),
        ]
        .spacing(s * 0.17)
        .align_x(iced::Alignment::Center);

        let w = size.width;
        let h = size.height;
        let r = size.width.min(size.height) * 0.15;

        let svg_data = format!(
            r#"<svg viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg">
        <path d="M0 {h1} Q0 {h} {r} {h} L0 {h} Z" fill="black"/>
        <path d="M{w1} {h} Q{w} {h} {w} {h1} L{w} {h} Z" fill="black"/>
        <path d="M{r} 0 Q0 0 0 {r} L0 0 Z" fill="black"/>
        <path d="M{w} {r} Q{w} 0 {w1} 0 L{w} 0 Z" fill="black"/>
        </svg>"#,
            w = w,
            h = h,
            h1 = h - r,
            w1 = w - r,
            r = r,
        );

        let corners = svg(svg::Handle::from_memory(svg_data.into_bytes()))
            .width(Length::Fill)
            .height(Length::Fill);

        stack![
            container(row![
                container(thumbnail)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(s * 0.1)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center),
                container(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::Center),
            ])
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(std::f32::consts::PI * 0.75)
                        .add_stop(0.0, gc1)
                        .add_stop(1.0, gc2),
                ))),
                ..Default::default()
            }),
            corners
        ]
        .into()
    }
}

#[derive(Debug, Clone)]
struct MediaMetadata {
    title: String,
    artist: String,
    position: i64,
    duration: i64,
    is_playing: bool,
    thumbnail: Option<iced::widget::image::Handle>,
    gradient_colors: Option<(Color, Color)>,
    position_origin: DateTime<Local>,
}

pub fn weekday_to_number(weekday: &Weekday) -> usize {
    match weekday {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        _ => 7,
    }
}

fn hand_rotation(n: u32, total: u32) -> Degrees {
    let turns = n as f32 / total as f32;

    Degrees(360.0 * turns)
}

fn hand_rotation_sec(value: f32, max: f32) -> Radians {
    Radians(value / max * std::f32::consts::TAU)
}

fn wmo_code_description(code: u8) -> &'static str {
    match code {
        0 => "Clear",
        1 => "Mostly clear",
        2 => "Partly cloudy",
        3 => "Cloudy",
        45..=48 => "Fog",
        51..=55 => "Drizzle",
        56..=57 => "Freezing drizzle",
        61..=63 => "Rain",
        65 => "Heavy rain",
        66..=67 => "Freezing rain",
        71..=73 => "Snow",
        75 => "Heavy snow",
        77 => "Blizzard",
        80..=86 => "Wintry mix",
        95..=99 => "Thunderstorm",
        _ => "n/a",
    }
}

fn wmo_code_svg(code: u8) -> &'static [u8] {
    match code {
        0 | 1 => include_bytes!("../icons/clear.svg"),
        // 1 => include_bytes!("../icons/mostly_clear.svg"),
        2 => include_bytes!("../icons/partly_cloudy.svg"),
        3 => include_bytes!("../icons/cloudy.svg"),
        45..=48 => include_bytes!("../icons/fog.svg"),
        51..=57 => include_bytes!("../icons/drizzle.svg"),
        // 56..=57 => include_bytes!("../icons/freezing_drizzle.svg"),
        61..=63 => include_bytes!("../icons/rain.svg"),
        65 => include_bytes!("../icons/heavy_rain.svg"),
        // 66..=67 => include_bytes!("../assets/weather/freezing_rain.svg"),
        71..=73 => include_bytes!("../icons/snow.svg"),
        // 75 => include_bytes!("../assets/weather/heavy_snow.svg"),
        // 77 => include_bytes!("../assets/weather/blizzard.svg"),
        // 80..=86 => include_bytes!("../assets/weather/wintry_mix.svg"),
        95..=99 => include_bytes!("../icons/thunderstorm.svg"),
        100 => include_bytes!("../icons/clear-night.svg"),
        _ => include_bytes!("../icons/warning.svg"),
    }
}

fn arrow_svg(direction: &str) -> &'static [u8] {
    match direction {
        "up" => include_bytes!("../icons/arrow-up-short.svg"),
        "down" => include_bytes!("../icons/arrow-down-short.svg"),
        "repeat" => include_bytes!("../icons/arrow-repeat.svg"),
        &_ => include_bytes!("../icons/arrow-down-short.svg"),
    }
}

fn lat_lon_to_xy(lat: f64, lon: f64, width: f32, height: f32) -> Point {
    let x = (lon + 180.0) / 360.0 * width as f64;

    let lat_rad = lat.to_radians();
    let merc = (lat_rad.tan() + 1.0 / lat_rad.cos()).ln();
    let y = (1.0 - merc / std::f64::consts::PI) / 2.0 * height as f64;

    Point::new(x as f32, y as f32)
}

fn extract_dominant_colors(buf: &[u8], theme_name: &str) -> (Color, Color) {
    let img = image::load_from_memory(buf).unwrap().to_rgb8();

    let pixels: Vec<[f32; 3]> = img
        .pixels()
        .step_by(10)
        .map(|p| {
            [
                p[0] as f32 / 255.0,
                p[1] as f32 / 255.0,
                p[2] as f32 / 255.0,
            ]
        })
        .collect();

    let mut c1 = pixels[0];
    let mut c2 = pixels[pixels.len() - 1];

    for _ in 0..20 {
        let mut sum1 = [0.0f32; 3];
        let mut sum2 = [0.0f32; 3];
        let mut count1 = 0usize;
        let mut count2 = 0usize;

        for p in &pixels {
            let d1 = dist(p, &c1);
            let d2 = dist(p, &c2);
            if d1 < d2 {
                sum1[0] += p[0];
                sum1[1] += p[1];
                sum1[2] += p[2];
                count1 += 1;
            } else {
                sum2[0] += p[0];
                sum2[1] += p[1];
                sum2[2] += p[2];
                count2 += 1;
            }
        }

        if count1 > 0 {
            c1 = [
                sum1[0] / count1 as f32,
                sum1[1] / count1 as f32,
                sum1[2] / count1 as f32,
            ];
        }
        if count2 > 0 {
            c2 = [
                sum2[0] / count2 as f32,
                sum2[1] / count2 as f32,
                sum2[2] / count2 as f32,
            ];
        }
    }

    let darken = |c: [f32; 3]| {
        if theme_name == "classic" {
            let min = 0.15f32;
            Color::from_rgb(
                (c[0] * 0.6).max(min),
                (c[1] * 0.6).max(min),
                (c[2] * 0.6).max(min),
            )
        } else {
            let luma = c[0] * 0.299 + c[1] * 0.587 + c[2] * 0.114;
            let l = (luma * 0.3).min(0.25);
            Color::from_rgb((l * 2.5).max(0.15).min(0.5), l * 0.3, l * 0.3)
        }
    };

    (darken(c1), darken(c2))
}

fn dist(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

struct SeekBar {
    progress: f32,
    preview: Option<f32>,
    bg_color: Color,
    bar_color: Color,
    radius: f32,
}

impl canvas::Program<Message> for SeekBar {
    type State = bool;

    fn update(
        &self,
        state: &mut bool,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    *state = true;
                    let ratio = (pos.x / bounds.width).clamp(0.0, 1.0);
                    return Some(canvas::Action::publish(Message::SeekPreview(ratio)));
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if *state {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let ratio = (pos.x / bounds.width).clamp(0.0, 1.0);

                        return Some(canvas::Action::publish(Message::SeekPreview(ratio)));
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if *state {
                    *state = false;
                    if let Some(pos) = cursor.position_in(bounds) {
                        let ratio = (pos.x / bounds.width).clamp(0.0, 1.0);
                        return Some(canvas::Action::publish(Message::SeekCommit(ratio)));
                    } else {
                        return Some(canvas::Action::publish(Message::SeekCommit(self.progress)));
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &bool,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let r = Radius::new(self.radius);

        frame.fill(
            &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), r),
            self.bg_color,
        );

        let progress = self.preview.unwrap_or(self.progress);
        frame.fill(
            &Path::rounded_rectangle(
                Point::ORIGIN,
                Size::new(bounds.width * progress, bounds.height),
                r,
            ),
            self.bar_color,
        );

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if *state {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

enum Orientation {
    Vertical,
    Horizontal,
}

struct VolumeBar {
    progress: f32,
    preview: Option<f32>,
    bg_color: Color,
    bar_color: Color,
    radius: f32,
    orientation: Orientation,
}

impl canvas::Program<Message> for VolumeBar {
    type State = bool;

    fn update(
        &self,
        state: &mut bool,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position_in(bounds) {
                    *state = true;
                    let ratio = calc_ratio(&self.orientation, pos, bounds);
                    return Some(canvas::Action::publish(Message::VolumePreview(ratio)));
                }
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if *state {
                    if let Some(pos) = cursor.position_in(bounds) {
                        let ratio = calc_ratio(&self.orientation, pos, bounds);
                        return Some(canvas::Action::publish(Message::VolumePreview(ratio)));
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if *state {
                    *state = false;
                    let ratio = if let Some(pos) = cursor.position_in(bounds) {
                        calc_ratio(&self.orientation, pos, bounds)
                    } else {
                        self.preview.unwrap_or(self.progress)
                    };
                    return Some(canvas::Action::publish(Message::VolumeCommit(ratio)));
                }
            }
            _ => {}
        }
        None
    }

    fn draw(
        &self,
        _state: &bool,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let r = Radius::new(self.radius);
        let progress = self.preview.unwrap_or(self.progress);

        match self.orientation {
            Orientation::Vertical => {
                let filled_h = bounds.height * progress;

                frame.fill(
                    &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), r),
                    self.bg_color,
                );

                frame.fill(
                    &Path::rounded_rectangle(
                        Point::new(0.0, bounds.height - filled_h),
                        Size::new(bounds.width, filled_h),
                        r,
                    ),
                    self.bar_color,
                );
                return vec![frame.into_geometry()];
            }

            Orientation::Horizontal => {
                let filled_w = bounds.width * progress;

                frame.fill(
                    &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), r),
                    self.bg_color,
                );

                frame.fill(
                    &Path::rounded_rectangle(Point::ORIGIN, Size::new(filled_w, bounds.height), r),
                    self.bar_color,
                );
                return vec![frame.into_geometry()];
            }
        }
    }

    fn mouse_interaction(
        &self,
        state: &bool,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if *state {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn calc_ratio(orientation: &Orientation, pos: Point, bounds: Rectangle) -> f32 {
    match orientation {
        Orientation::Vertical => 1.0 - (pos.y / bounds.height * 1.05).clamp(0.0, 1.0),
        Orientation::Horizontal => (pos.x / bounds.width * 1.05).clamp(0.0, 1.0),
    }
}

pub fn subsolar_point() -> (f64, f64) {
    let utc = Utc::now();
    let day = utc.ordinal() as f64;

    let declination = (23.45_f64.to_radians().sin()
        * ((360.0 / 365.0 * (day - 81.0)).to_radians()).sin())
    .asin()
    .to_degrees();

    let b = (360.0 / 365.0 * (day - 81.0)).to_radians();
    let eot = 9.87 * (2.0 * b).sin() - 7.53 * b.cos() - 1.5 * b.sin();

    let hours_utc =
        utc.hour() as f64 + utc.minute() as f64 / 60.0 + utc.second() as f64 / 3600.0 + eot / 60.0;

    let subsolar_lon = (12.0 - hours_utc) * 15.0;
    let subsolar_lon = ((subsolar_lon + 180.0).rem_euclid(360.0)) - 180.0;

    (declination, subsolar_lon)
}

pub fn terminator_points(sub_lat_deg: f64, sub_lon_deg: f64, n: usize) -> Vec<(f64, f64)> {
    let dec = sub_lat_deg.to_radians();
    let ra = sub_lon_deg.to_radians();

    let nx = dec.cos() * ra.cos();
    let ny = dec.cos() * ra.sin();
    let nz = dec.sin();

    let mag = (nx * nx + ny * ny).sqrt();
    let (v1x, v1y, v1z) = if mag > 1e-6 {
        (-ny / mag, nx / mag, 0.0)
    } else {
        (1.0, 0.0, 0.0)
    };

    let v2x = ny * v1z - nz * v1y;
    let v2y = nz * v1x - nx * v1z;
    let v2z = nx * v1y - ny * v1x;

    (0..=n)
        .map(|i| {
            let t = 2.0 * PI * i as f64 / n as f64;
            let px = t.cos() * v1x + t.sin() * v2x;
            let py = t.cos() * v1y + t.sin() * v2y;
            let pz = t.cos() * v1z + t.sin() * v2z;

            let lat = pz.asin().to_degrees();
            let lon = py.atan2(px).to_degrees();
            (lat, lon)
        })
        .collect()
}
pub fn draw_night_overlay(frame: &mut Frame, bounds: Rectangle, sub_lat: f64, sub_lon: f64) {
    let mut points = terminator_points(sub_lat, sub_lon, 360);
    points.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let to_canvas = |(lat, lon): (f64, f64)| -> Point {
        let x = bounds.x + ((lon + 180.0) / 360.0) as f32 * bounds.width;
        let y = bounds.y + ((90.0 - lat) / 180.0) as f32 * bounds.height;
        Point::new(x, y)
    };

    for offset in [-bounds.width, 0.0, bounds.width] {
        for i in 0..points.len() - 1 {
            let (lat0, lon0) = points[i];
            let (_, lon1) = points[i + 1];

            if (lon1 - lon0).abs() > 180.0 {
                continue;
            }

            let mut p0 = to_canvas(points[i]);
            let mut p1 = to_canvas(points[i + 1]);
            p0.x += offset;
            p1.x += offset;

            let t = (lat0.abs() / 90.0) as f32;
            let color = Color::from_rgba(1.0, 1.0, 1.0, 0.3 + t * 0.5);

            frame.stroke(
                &Path::new(|b| {
                    b.move_to(p0);
                    b.line_to(p1);
                }),
                canvas::Stroke::default()
                    .with_color(color)
                    .with_width(bounds.width.min(bounds.height) * 0.015),
            );
        }

        let mut last_pt = to_canvas(*points.last().unwrap());
        let mut next_first_pt = to_canvas(*points.first().unwrap());
        last_pt.x += offset;
        next_first_pt.x += offset + bounds.width;

        let t = (points.last().unwrap().0.abs() / 90.0) as f32;
        frame.stroke(
            &Path::new(|b| {
                b.move_to(last_pt);
                b.line_to(next_first_pt);
            }),
            canvas::Stroke::default()
                .with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.3 + t * 0.5))
                .with_width(bounds.width.min(bounds.height) * 0.015),
        );
    }
}

static MAP_DOTS: &[(f64, f64)] = &[
    (-87.85016286644952, -69.45013979496738),
    (-73.77850162866449, -69.45013979496738),
    (-73.77850162866449, -63.41099720410065),
    (-66.74267100977201, -63.41099720410065),
    (-66.74267100977201, -57.53960857409133),
    (-59.70684039087948, -57.53960857409133),
    (-59.70684039087948, -51.500465983224615),
    (-52.671009771986974, -51.500465983224615),
    (-52.671009771986974, -45.46132339235788),
    (-45.63517915309447, -45.46132339235788),
    (-45.63517915309447, -39.42218080149115),
    (-38.599348534201965, -39.42218080149115),
    (-38.599348534201965, -45.46132339235788),
    (-38.599348534201965, -51.500465983224615),
    (-45.63517915309447, -51.500465983224615),
    (-45.63517915309447, -57.53960857409133),
    (-52.671009771986974, -57.53960857409133),
    (-52.671009771986974, -63.41099720410065),
    (-52.671009771986974, -69.28238583410996),
    (-52.671009771986974, -69.45013979496738),
    (-45.63517915309447, -69.45013979496738),
    (-45.63517915309447, -63.41099720410065),
    (-38.599348534201965, -63.41099720410065),
    (-38.599348534201965, -57.53960857409133),
    (-31.563517915309447, -57.53960857409133),
    (-31.563517915309447, -51.500465983224615),
    (-31.563517915309447, -45.46132339235788),
    (-31.563517915309447, -39.42218080149115),
    (-31.563517915309447, -33.38303821062442),
    (-24.52768729641693, -33.38303821062442),
    (-24.52768729641693, -39.42218080149115),
    (-24.52768729641693, -45.46132339235788),
    (-24.52768729641693, -51.500465983224615),
    (-24.52768729641693, -57.53960857409133),
    (-24.52768729641693, -63.41099720410065),
    (-31.563517915309447, -63.41099720410065),
    (-31.563517915309447, -69.45013979496738),
    (-38.599348534201965, -69.45013979496738),
    (-38.599348534201965, -75.48928238583412),
    (-31.563517915309447, -75.48928238583412),
    (-31.563517915309447, -81.52842497670085),
    (-24.52768729641693, -81.52842497670085),
    (-24.52768729641693, -75.48928238583412),
    (-24.52768729641693, -69.45013979496738),
    (-17.49185667752444, -69.45013979496738),
    (-17.49185667752444, -63.41099720410065),
    (-17.49185667752444, -57.53960857409133),
    (-17.49185667752444, -51.500465983224615),
    (-17.49185667752444, -45.46132339235788),
    (-10.45602605863192, -51.500465983224615),
    (-10.45602605863192, -57.53960857409133),
    (-10.45602605863192, -63.41099720410065),
    (-3.420195439739416, -63.41099720410065),
    (-3.420195439739416, -69.45013979496738),
    (-10.45602605863192, -69.45013979496738),
    (-10.45602605863192, -75.48928238583412),
    (-17.49185667752444, -75.48928238583412),
    (-17.49185667752444, -81.52842497670085),
    (-3.420195439739416, -75.48928238583412),
    (-3.420195439739416, -81.52842497670085),
    (-3.420195439739416, -87.56756756756758),
    (-59.70684039087948, -63.41099720410065),
    (-59.70684039087948, -69.45013979496738),
    (-66.74267100977201, -69.45013979496738),
    (-73.77850162866449, -75.48928238583412),
    (-80.81433224755699, -75.48928238583412),
    (-80.81433224755699, -69.45013979496738),
    (-87.85016286644952, -69.45013979496738),
    (-73.77850162866449, -69.45013979496738),
    (-73.77850162866449, -63.41099720410065),
    (-66.74267100977201, -63.41099720410065),
    (-66.74267100977201, -57.53960857409133),
    (-59.70684039087948, -57.53960857409133),
    (-59.70684039087948, -51.500465983224615),
    (-52.671009771986974, -51.500465983224615),
    (-52.671009771986974, -45.46132339235788),
    (-45.63517915309447, -45.46132339235788),
    (-45.63517915309447, -39.42218080149115),
    (-38.599348534201965, -39.42218080149115),
    (-38.599348534201965, -45.46132339235788),
    (-38.599348534201965, -51.500465983224615),
    (-45.63517915309447, -51.500465983224615),
    (-45.63517915309447, -57.53960857409133),
    (-52.671009771986974, -57.53960857409133),
    (-52.671009771986974, -63.41099720410065),
    (-52.671009771986974, -69.28238583410996),
    (-52.671009771986974, -69.45013979496738),
    (-45.63517915309447, -69.45013979496738),
    (-45.63517915309447, -63.41099720410065),
    (-38.599348534201965, -63.41099720410065),
    (-38.599348534201965, -57.53960857409133),
    (-31.563517915309447, -57.53960857409133),
    (-31.563517915309447, -51.500465983224615),
    (-31.563517915309447, -45.46132339235788),
    (-31.563517915309447, -39.42218080149115),
    (-31.563517915309447, -33.38303821062442),
    (-24.52768729641693, -33.38303821062442),
    (-24.52768729641693, -39.42218080149115),
    (-24.52768729641693, -45.46132339235788),
    (-24.52768729641693, -51.500465983224615),
    (-24.52768729641693, -57.53960857409133),
    (-24.52768729641693, -63.41099720410065),
    (-31.563517915309447, -63.41099720410065),
    (-31.563517915309447, -69.45013979496738),
    (-38.599348534201965, -69.45013979496738),
    (-38.599348534201965, -75.48928238583412),
    (-31.563517915309447, -75.48928238583412),
    (-31.563517915309447, -81.52842497670085),
    (-24.52768729641693, -81.52842497670085),
    (-24.52768729641693, -75.48928238583412),
    (-24.52768729641693, -69.45013979496738),
    (-17.49185667752444, -69.45013979496738),
    (-17.49185667752444, -63.41099720410065),
    (-17.49185667752444, -57.53960857409133),
    (-17.49185667752444, -51.500465983224615),
    (-17.49185667752444, -45.46132339235788),
    (-10.45602605863192, -51.500465983224615),
    (-10.45602605863192, -57.53960857409133),
    (-10.45602605863192, -63.41099720410065),
    (-3.420195439739416, -63.41099720410065),
    (-3.420195439739416, -69.45013979496738),
    (-10.45602605863192, -69.45013979496738),
    (-10.45602605863192, -75.48928238583412),
    (-17.49185667752444, -75.48928238583412),
    (-17.49185667752444, -81.52842497670085),
    (-3.420195439739416, -75.48928238583412),
    (-3.420195439739416, -81.52842497670085),
    (-3.420195439739416, -87.56756756756758),
    (-59.70684039087948, -63.41099720410065),
    (-59.70684039087948, -69.45013979496738),
    (-66.74267100977201, -69.45013979496738),
    (-73.77850162866449, -75.48928238583412),
    (-80.81433224755699, -75.48928238583412),
    (-80.81433224755699, -69.45013979496738),
    (-87.85016286644952, -69.45013979496738),
    (-73.77850162866449, -69.45013979496738),
    (-73.77850162866449, -63.41099720410065),
    (-66.74267100977201, -63.41099720410065),
    (-66.74267100977201, -57.53960857409133),
    (-59.70684039087948, -57.53960857409133),
    (-59.70684039087948, -51.500465983224615),
    (-52.671009771986974, -51.500465983224615),
    (-52.671009771986974, -45.46132339235788),
    (-45.63517915309447, -45.46132339235788),
    (-45.63517915309447, -39.42218080149115),
    (-38.599348534201965, -39.42218080149115),
    (-38.599348534201965, -45.46132339235788),
    (-38.599348534201965, -51.500465983224615),
    (-45.63517915309447, -51.500465983224615),
    (-45.63517915309447, -57.53960857409133),
    (-52.671009771986974, -57.53960857409133),
    (-52.671009771986974, -63.41099720410065),
    (-52.671009771986974, -69.28238583410996),
    (-52.671009771986974, -69.45013979496738),
    (-45.63517915309447, -69.45013979496738),
    (-45.63517915309447, -63.41099720410065),
    (-38.599348534201965, -63.41099720410065),
    (-38.599348534201965, -57.53960857409133),
    (-31.563517915309447, -57.53960857409133),
    (-31.563517915309447, -51.500465983224615),
    (-31.563517915309447, -45.46132339235788),
    (-31.563517915309447, -39.42218080149115),
    (-31.563517915309447, -33.38303821062442),
    (-24.52768729641693, -33.38303821062442),
    (-24.52768729641693, -39.42218080149115),
    (-24.52768729641693, -45.46132339235788),
    (-24.52768729641693, -51.500465983224615),
    (-24.52768729641693, -57.53960857409133),
    (-24.52768729641693, -63.41099720410065),
    (-31.563517915309447, -63.41099720410065),
    (-31.563517915309447, -69.45013979496738),
    (-38.599348534201965, -69.45013979496738),
    (-38.599348534201965, -75.48928238583412),
    (-31.563517915309447, -75.48928238583412),
    (-31.563517915309447, -81.52842497670085),
    (-24.52768729641693, -81.52842497670085),
    (-24.52768729641693, -75.48928238583412),
    (-24.52768729641693, -69.45013979496738),
    (-17.49185667752444, -69.45013979496738),
    (-17.49185667752444, -63.41099720410065),
    (-17.49185667752444, -57.53960857409133),
    (-17.49185667752444, -51.500465983224615),
    (-17.49185667752444, -45.46132339235788),
    (-10.45602605863192, -51.500465983224615),
    (-10.45602605863192, -57.53960857409133),
    (-10.45602605863192, -63.41099720410065),
    (-3.420195439739416, -63.41099720410065),
    (-3.420195439739416, -69.45013979496738),
    (-10.45602605863192, -69.45013979496738),
    (-10.45602605863192, -75.48928238583412),
    (-17.49185667752444, -75.48928238583412),
    (-17.49185667752444, -81.52842497670085),
    (-3.420195439739416, -75.48928238583412),
    (-3.420195439739416, -81.52842497670085),
    (-3.420195439739416, -87.56756756756758),
    (-59.70684039087948, -63.41099720410065),
    (-59.70684039087948, -69.45013979496738),
    (-66.74267100977201, -69.45013979496738),
    (-73.77850162866449, -75.48928238583412),
    (-80.81433224755699, -75.48928238583412),
    (-80.81433224755699, -69.45013979496738),
    (-87.85016286644952, -69.45013979496738),
    (-73.77850162866449, -69.45013979496738),
    (-73.77850162866449, -63.41099720410065),
    (-66.74267100977201, -63.41099720410065),
    (-66.74267100977201, -57.53960857409133),
    (-59.70684039087948, -57.53960857409133),
    (-59.70684039087948, -51.500465983224615),
    (-52.671009771986974, -51.500465983224615),
    (-52.671009771986974, -45.46132339235788),
    (-45.63517915309447, -45.46132339235788),
    (-45.63517915309447, -39.42218080149115),
    (-38.599348534201965, -39.42218080149115),
    (-38.599348534201965, -45.46132339235788),
    (-38.599348534201965, -51.500465983224615),
    (-45.63517915309447, -51.500465983224615),
    (-45.63517915309447, -57.53960857409133),
    (-52.671009771986974, -57.53960857409133),
    (-52.671009771986974, -63.41099720410065),
    (-52.671009771986974, -69.28238583410996),
    (-52.671009771986974, -69.45013979496738),
    (-45.63517915309447, -69.45013979496738),
    (-45.63517915309447, -63.41099720410065),
    (-38.599348534201965, -63.41099720410065),
    (-38.599348534201965, -57.53960857409133),
    (-31.563517915309447, -57.53960857409133),
    (-31.563517915309447, -51.500465983224615),
    (-31.563517915309447, -45.46132339235788),
    (-31.563517915309447, -39.42218080149115),
    (-31.563517915309447, -33.38303821062442),
    (-24.52768729641693, -33.38303821062442),
    (-24.52768729641693, -39.42218080149115),
    (-24.52768729641693, -45.46132339235788),
    (-24.52768729641693, -51.500465983224615),
    (-24.52768729641693, -57.53960857409133),
    (-24.52768729641693, -63.41099720410065),
    (-31.563517915309447, -63.41099720410065),
    (-31.563517915309447, -69.45013979496738),
    (-38.599348534201965, -69.45013979496738),
    (-38.599348534201965, -75.48928238583412),
    (-31.563517915309447, -75.48928238583412),
    (-31.563517915309447, -81.52842497670085),
    (-24.52768729641693, -81.52842497670085),
    (-24.52768729641693, -75.48928238583412),
    (-24.52768729641693, -69.45013979496738),
    (-17.49185667752444, -69.45013979496738),
    (-17.49185667752444, -63.41099720410065),
    (-17.49185667752444, -57.53960857409133),
    (-17.49185667752444, -51.500465983224615),
    (-17.49185667752444, -45.46132339235788),
    (-10.45602605863192, -51.500465983224615),
    (-10.45602605863192, -57.53960857409133),
    (-10.45602605863192, -63.41099720410065),
    (-3.420195439739416, -63.41099720410065),
    (-3.420195439739416, -69.45013979496738),
    (-10.45602605863192, -69.45013979496738),
    (-10.45602605863192, -75.48928238583412),
    (-17.49185667752444, -75.48928238583412),
    (-17.49185667752444, -81.52842497670085),
    (-3.420195439739416, -75.48928238583412),
    (-3.420195439739416, -81.52842497670085),
    (-3.420195439739416, -87.56756756756758),
    (3.6156351791531023, -87.56756756756758),
    (3.6156351791531023, -93.6067101584343),
    (3.6156351791531023, -99.64585274930103),
    (10.651465798045606, -99.64585274930103),
    (10.651465798045606, -105.68499534016776),
    (17.68729641693811, -105.68499534016776),
    (17.68729641693811, -111.89189189189189),
    (24.723127035830615, -111.89189189189189),
    (24.723127035830615, -117.76328052190121),
    (31.563517915309447, -117.76328052190121),
    (31.563517915309447, -123.80242311276794),
    (38.990228013029316, -123.80242311276794),
    (38.990228013029316, -117.76328052190121),
    (38.990228013029316, -111.89189189189189),
    (38.990228013029316, -105.68499534016776),
    (38.990228013029316, -99.64585274930103),
    (38.990228013029316, -93.6067101584343),
    (38.990228013029316, -87.56756756756758),
    (38.990228013029316, -81.52842497670085),
    (38.990228013029316, -75.48928238583412),
    (31.563517915309447, -75.48928238583412),
    (31.563517915309447, -81.52842497670085),
    (31.563517915309447, -87.56756756756758),
    (31.563517915309447, -93.6067101584343),
    (31.563517915309447, -99.64585274930103),
    (31.563517915309447, -105.68499534016776),
    (31.758957654723133, -111.89189189189189),
    (24.723127035830615, -105.68499534016776),
    (24.723127035830615, -99.64585274930103),
    (24.723127035830615, -93.6067101584343),
    (24.723127035830615, -87.56756756756758),
    (24.723127035830615, -81.52842497670085),
    (17.68729641693811, -99.64585274930103),
    (17.68729641693811, -81.52842497670085),
    (10.651465798045606, -81.52842497670085),
    (10.651465798045606, -87.56756756756758),
    (10.651465798045606, -75.48928238583412),
    (10.651465798045606, -69.45013979496738),
    (38.990228013029316, -69.45013979496738),
    (38.990228013029316, -63.41099720410065),
    (38.990228013029316, -57.53960857409133),
    (46.02605863192183, -57.53960857409133),
    (46.02605863192183, -51.500465983224615),
    (46.02605863192183, -63.41099720410065),
    (46.02605863192183, -69.45013979496738),
    (46.02605863192183, -75.48928238583412),
    (46.02605863192183, -81.52842497670085),
    (46.02605863192183, -87.56756756756758),
    (46.02605863192183, -93.6067101584343),
    (46.02605863192183, -99.64585274930103),
    (46.02605863192183, -105.68499534016776),
    (46.02605863192183, -111.89189189189189),
    (46.02605863192183, -117.76328052190121),
    (46.02605863192183, -123.80242311276794),
    (46.02605863192183, -129.67381174277727),
    (52.86644951140065, -129.67381174277727),
    (52.86644951140065, -135.8807082945014),
    (59.902280130293164, -135.8807082945014),
    (59.902280130293164, -141.91985088536813),
    (59.902280130293164, -147.95899347623487),
    (59.902280130293164, -153.9981360671016),
    (59.902280130293164, -160.03727865796833),
    (59.902280130293164, -166.07642124883503),
    (52.86644951140065, -166.07642124883503),
    (52.86644951140065, -172.2833178005592),
    (66.93811074918567, -172.2833178005592),
    (66.93811074918567, -178.1547064305685),
    (66.93811074918567, -166.07642124883503),
    (66.93811074918567, -160.03727865796833),
    (73.97394136807817, -147.95899347623487),
    (73.97394136807817, -141.91985088536813),
    (73.97394136807817, -135.8807082945014),
    (66.93811074918567, -135.71295433364398),
    (66.93811074918567, -129.84156570363467),
    (73.97394136807817, -129.67381174277727),
    (73.97394136807817, -123.80242311276794),
    (81.00977198697069, -123.80242311276794),
    (81.00977198697069, -117.76328052190121),
    (81.00977198697069, -111.89189189189189),
    (73.97394136807817, -111.89189189189189),
    (73.97394136807817, -117.76328052190121),
    (66.93811074918567, -117.76328052190121),
    (66.93811074918567, -123.80242311276794),
    (59.902280130293164, -123.80242311276794),
    (59.902280130293164, -129.67381174277727),
    (52.86644951140065, -123.80242311276794),
    (52.86644951140065, -117.76328052190121),
    (59.902280130293164, -117.76328052190121),
    (59.902280130293164, -111.89189189189189),
    (66.93811074918567, -111.89189189189189),
    (66.93811074918567, -105.68499534016776),
    (59.902280130293164, -105.68499534016776),
    (52.86644951140065, -105.68499534016776),
    (52.86644951140065, -111.89189189189189),
    (73.97394136807817, -105.68499534016776),
    (81.00977198697069, -105.68499534016776),
    (81.00977198697069, -99.64585274930103),
    (81.00977198697069, -93.6067101584343),
    (87.85016286644951, -93.6067101584343),
    (87.85016286644951, -87.56756756756758),
    (87.85016286644951, -81.52842497670085),
    (81.00977198697069, -81.52842497670085),
    (81.00977198697069, -87.56756756756758),
    (73.97394136807817, -87.56756756756758),
    (66.93811074918567, -87.56756756756758),
    (73.97394136807817, -93.6067101584343),
    (73.97394136807817, -99.64585274930103),
    (66.93811074918567, -99.64585274930103),
    (66.93811074918567, -93.6067101584343),
    (59.902280130293164, -93.6067101584343),
    (59.902280130293164, -99.64585274930103),
    (52.86644951140065, -99.64585274930103),
    (52.86644951140065, -93.6067101584343),
    (52.86644951140065, -87.56756756756758),
    (73.97394136807817, -81.52842497670085),
    (73.97394136807817, -75.48928238583412),
    (87.85016286644951, -75.48928238583412),
    (87.85016286644951, -69.45013979496738),
    (87.85016286644951, -63.41099720410065),
    (81.00977198697069, -63.41099720410065),
    (81.00977198697069, -57.53960857409133),
    (87.85016286644951, -57.53960857409133),
    (87.85016286644951, -51.500465983224615),
    (81.00977198697069, -51.500465983224615),
    (73.97394136807817, -51.500465983224615),
    (66.93811074918567, -51.500465983224615),
    (66.93811074918567, -45.46132339235788),
    (59.902280130293164, -45.29356943150049),
    (59.902280130293164, -39.254426840633755),
    (66.93811074918567, -39.254426840633755),
    (73.97394136807817, -39.254426840633755),
    (73.97394136807817, -33.21528424976702),
    (73.97394136807817, -27.176141658900292),
    (66.93811074918567, -33.21528424976702),
    (73.97394136807817, -45.29356943150049),
    (81.00977198697069, -45.29356943150049),
    (87.85016286644951, -45.29356943150049),
    (87.85016286644951, -39.254426840633755),
    (87.85016286644951, -33.21528424976702),
    (87.85016286644951, -27.176141658900292),
    (87.85016286644951, -21.13699906803356),
    (87.85016286644951, -15.265610438024225),
    (87.85016286644951, -9.058713886300097),
    (87.85016286644951, 15.0978564771668),
    (66.93811074918567, -15.265610438024225),
    (87.85016286644951, 21.13699906803356),
    (66.93811074918567, 21.13699906803356),
    (59.902280130293164, 21.13699906803356),
    (52.86644951140065, 21.13699906803356),
    (46.02605863192183, 21.13699906803356),
    (38.990228013029316, 21.13699906803356),
    (31.563517915309447, 21.13699906803356),
    (31.563517915309447, 27.176141658900264),
    (31.563517915309447, 33.21528424976702),
    (31.563517915309447, 39.25442684063373),
    (24.723127035830615, 39.25442684063373),
    (24.723127035830615, 45.29356943150049),
    (31.563517915309447, 45.29356943150049),
    (38.990228013029316, 45.29356943150049),
    (46.02605863192183, 45.29356943150049),
    (52.86644951140065, 45.29356943150049),
    (59.902280130293164, 45.29356943150049),
    (66.93811074918567, 45.29356943150049),
    (66.93811074918567, 51.16495806150979),
    (73.97394136807817, 51.16495806150979),
    (73.97394136807817, 57.204100652376525),
    (81.00977198697069, 57.204100652376525),
    (81.00977198697069, 63.410997204100624),
    (81.00977198697069, 69.45013979496738),
    (73.97394136807817, 69.45013979496738),
    (66.93811074918567, 69.45013979496738),
    (66.93811074918567, 63.410997204100624),
    (66.93811074918567, 57.37185461323392),
    (59.902280130293164, 57.204100652376525),
    (59.902280130293164, 51.33271202236719),
    (52.86644951140065, 51.33271202236719),
    (52.86644951140065, 57.204100652376525),
    (59.902280130293164, 63.410997204100624),
    (59.902280130293164, 69.45013979496738),
    (52.86644951140065, 69.45013979496738),
    (52.86644951140065, 63.410997204100624),
    (46.02605863192183, 63.410997204100624),
    (46.02605863192183, 69.45013979496738),
    (45.83061889250814, 57.204100652376525),
    (46.02605863192183, 51.33271202236719),
    (38.990228013029316, 51.16495806150979),
    (31.563517915309447, 51.16495806150979),
    (38.79478827361563, 57.204100652376525),
    (38.990228013029316, 63.410997204100624),
    (38.990228013029316, 69.45013979496738),
    (31.563517915309447, 69.45013979496738),
    (31.563517915309447, 63.410997204100624),
    (24.723127035830615, 63.410997204100624),
    (24.723127035830615, 57.204100652376525),
    (24.723127035830615, 51.33271202236719),
    (31.758957654723133, 57.204100652376525),
    (38.990228013029316, 27.176141658900264),
    (46.02605863192183, 27.176141658900264),
    (52.86644951140065, 27.176141658900264),
    (59.902280130293164, 27.176141658900264),
    (66.93811074918567, 27.176141658900264),
    (73.97394136807817, 27.176141658900264),
    (73.97394136807817, 33.21528424976702),
    (66.93811074918567, 33.21528424976702),
    (59.902280130293164, 33.21528424976702),
    (52.86644951140065, 33.21528424976702),
    (46.02605863192183, 33.21528424976702),
    (46.02605863192183, 39.25442684063373),
    (52.86644951140065, 39.25442684063373),
    (59.902280130293164, 39.25442684063373),
    (66.93811074918567, 39.25442684063373),
    (73.97394136807817, 15.0978564771668),
    (66.93811074918567, 15.0978564771668),
    (59.902280130293164, 15.0978564771668),
    (59.902280130293164, 9.058713886300097),
    (52.86644951140065, 9.058713886300097),
    (52.86644951140065, 15.0978564771668),
    (46.02605863192183, 15.0978564771668),
    (46.02605863192183, 9.058713886300097),
    (46.02605863192183, 3.0195712954333658),
    (46.02605863192183, -3.0195712954333658),
    (52.86644951140065, -3.0195712954333658),
    (52.86644951140065, -9.058713886300097),
    (38.990228013029316, -3.0195712954333658),
    (38.990228013029316, -9.058713886300097),
    (38.990228013029316, 3.0195712954333658),
    (38.990228013029316, 9.058713886300097),
    (38.990228013029316, 15.0978564771668),
    (31.563517915309447, 15.0978564771668),
    (31.563517915309447, -3.0195712954333658),
    (31.563517915309447, -9.058713886300097),
    (24.723127035830615, -3.0195712954333658),
    (17.68729641693811, -3.0195712954333658),
    (17.68729641693811, -9.058713886300097),
    (17.68729641693811, 3.0195712954333658),
    (17.68729641693811, 9.058713886300097),
    (17.68729641693811, 15.0978564771668),
    (17.68729641693811, 21.13699906803356),
    (17.68729641693811, 27.176141658900264),
    (17.68729641693811, 33.21528424976702),
    (17.68729641693811, 39.25442684063373),
    (17.68729641693811, 45.29356943150049),
    (17.68729641693811, 51.33271202236719),
    (17.68729641693811, 57.204100652376525),
    (17.68729641693811, 63.410997204100624),
    (24.723127035830615, 69.45013979496738),
    (17.68729641693811, 69.45013979496738),
    (10.651465798045606, 69.45013979496738),
    (73.97394136807817, 75.48928238583409),
    (66.93811074918567, 75.48928238583409),
    (59.902280130293164, 75.48928238583409),
    (52.86644951140065, 75.48928238583409),
    (46.02605863192183, 75.48928238583409),
    (38.990228013029316, 75.48928238583409),
    (31.563517915309447, 75.48928238583409),
    (24.723127035830615, 75.48928238583409),
    (17.68729641693811, 75.48928238583409),
    (10.651465798045606, 75.48928238583409),
    (73.97394136807817, 81.52842497670082),
    (66.93811074918567, 81.52842497670082),
    (59.902280130293164, 81.52842497670082),
    (52.86644951140065, 81.52842497670082),
    (46.02605863192183, 81.52842497670082),
    (38.990228013029316, 81.52842497670082),
    (31.563517915309447, 81.52842497670082),
    (24.723127035830615, 81.52842497670082),
    (17.68729641693811, 81.52842497670082),
    (10.651465798045606, 81.52842497670082),
    (3.6156351791531023, 75.48928238583409),
    (-3.420195439739416, 75.48928238583409),
    (3.6156351791531023, 81.52842497670082),
    (-3.420195439739416, 81.52842497670082),
    (73.97394136807817, 87.56756756756755),
    (66.93811074918567, 87.56756756756755),
    (59.902280130293164, 87.56756756756755),
    (52.86644951140065, 87.56756756756755),
    (46.02605863192183, 87.56756756756755),
    (38.990228013029316, 87.56756756756755),
    (31.563517915309447, 87.56756756756755),
    (24.723127035830615, 87.56756756756755),
    (17.68729641693811, 87.56756756756755),
    (10.651465798045606, 87.56756756756755),
    (3.6156351791531023, 87.56756756756755),
    (-10.45602605863192, 81.52842497670082),
    (73.97394136807817, 93.60671015843428),
    (66.93811074918567, 93.60671015843428),
    (59.902280130293164, 93.60671015843428),
    (52.86644951140065, 93.60671015843428),
    (46.02605863192183, 93.60671015843428),
    (38.990228013029316, 93.60671015843428),
    (31.563517915309447, 93.60671015843428),
    (24.723127035830615, 93.60671015843428),
    (17.68729641693811, 93.60671015843428),
    (10.651465798045606, 93.60671015843428),
    (73.97394136807817, 99.64585274930101),
    (66.93811074918567, 99.64585274930101),
    (59.902280130293164, 99.64585274930101),
    (52.86644951140065, 99.64585274930101),
    (46.02605863192183, 99.64585274930101),
    (38.990228013029316, 99.64585274930101),
    (31.563517915309447, 99.64585274930101),
    (24.723127035830615, 99.64585274930101),
    (17.68729641693811, 99.64585274930101),
    (10.651465798045606, 99.64585274930101),
    (73.97394136807817, 105.68499534016775),
    (66.93811074918567, 105.68499534016775),
    (59.902280130293164, 105.68499534016775),
    (52.86644951140065, 105.68499534016775),
    (46.02605863192183, 105.68499534016775),
    (38.990228013029316, 105.68499534016775),
    (31.563517915309447, 105.68499534016775),
    (24.723127035830615, 105.68499534016775),
    (17.68729641693811, 105.68499534016775),
    (10.651465798045606, 105.68499534016775),
    (-45.63517915309447, 117.76328052190121),
    (-52.671009771986974, 117.76328052190121),
    (-59.70684039087948, 117.76328052190121),
    (-45.63517915309447, 123.80242311276794),
    (-52.671009771986974, 123.80242311276794),
    (-59.70684039087948, 123.80242311276794),
    (-45.63517915309447, 129.84156570363467),
    (-52.671009771986974, 129.84156570363467),
    (-59.70684039087948, 129.84156570363467),
    (-45.63517915309447, 135.8807082945014),
    (-52.671009771986974, 135.8807082945014),
    (-59.70684039087948, 135.8807082945014),
    (-45.63517915309447, 141.91985088536813),
    (-52.671009771986974, 141.91985088536813),
    (-59.70684039087948, 141.91985088536813),
    (-45.63517915309447, 147.9589934762348),
    (-38.599348534201965, 129.84156570363467),
    (-38.599348534201965, 135.8807082945014),
    (-31.563517915309447, 135.8807082945014),
    (-24.52768729641693, 141.91985088536813),
    (-17.49185667752444, 141.91985088536813),
    (-24.52768729641693, 147.9589934762348),
    (-24.52768729641693, 153.9981360671016),
    (-31.563517915309447, 153.9981360671016),
    (-31.563517915309447, 147.9589934762348),
    (-66.74267100977201, 147.9589934762348),
    (-66.74267100977201, 141.91985088536813),
    (-66.74267100977201, 178.15470643056852),
    (-59.70684039087948, 178.15470643056852),
    (-73.77850162866449, 178.15470643056852),
    (-73.77850162866449, 172.1155638397018),
    (-80.61889250814332, 172.1155638397018),
    (-38.599348534201965, 141.91985088536813),
    (-38.599348534201965, 147.9589934762348),
    (-52.671009771986974, 147.9589934762348),
    (-59.70684039087948, 147.9589934762348),
    (-45.63517915309447, 153.9981360671016),
    (-52.671009771986974, 153.9981360671016),
    (-59.70684039087948, 153.9981360671016),
    (73.97394136807817, 111.72413793103448),
    (66.93811074918567, 111.72413793103448),
    (59.902280130293164, 111.72413793103448),
    (52.86644951140065, 111.72413793103448),
    (46.02605863192183, 111.72413793103448),
    (38.990228013029316, 111.72413793103448),
    (31.563517915309447, 111.72413793103448),
    (24.723127035830615, 111.72413793103448),
    (17.68729641693811, 111.72413793103448),
    (10.651465798045606, 111.72413793103448),
    (73.97394136807817, 123.80242311276794),
    (66.93811074918567, 123.80242311276794),
    (59.902280130293164, 123.80242311276794),
    (52.86644951140065, 123.80242311276794),
    (46.02605863192183, 123.80242311276794),
    (38.990228013029316, 123.80242311276794),
    (31.563517915309447, 123.80242311276794),
    (31.563517915309447, 129.84156570363467),
    (31.563517915309447, 141.91985088536813),
    (24.723127035830615, 141.91985088536813),
    (24.723127035830615, 135.8807082945014),
    (-3.420195439739416, 129.84156570363467),
    (-3.420195439739416, 123.80242311276794),
    (-10.45602605863192, 129.84156570363467),
    (-10.45602605863192, 123.80242311276794),
    (3.6156351791531023, 111.72413793103448),
    (-3.420195439739416, 111.72413793103448),
    (-10.45602605863192, 117.76328052190121),
    (3.6156351791531023, 105.68499534016775),
    (3.6156351791531023, 99.64585274930101),
    (-3.420195439739416, 99.64585274930101),
    (-10.45602605863192, 99.64585274930101),
    (-3.420195439739416, 105.68499534016775),
    (-17.49185667752444, 123.80242311276794),
    (-24.52768729641693, 123.80242311276794),
    (-17.49185667752444, 117.76328052190121),
    (-17.49185667752444, 111.72413793103448),
    (-24.52768729641693, 111.72413793103448),
    (-24.52768729641693, 117.76328052190121),
    (-10.45602605863192, 105.68499534016775),
    (-17.49185667752444, 105.68499534016775),
    (-17.49185667752444, 99.64585274930101),
    (24.723127035830615, 123.80242311276794),
    (17.68729641693811, 123.80242311276794),
    (73.97394136807817, 117.76328052190121),
    (80.814332247557, 93.60671015843428),
    (80.814332247557, 99.64585274930101),
    (80.814332247557, 105.68499534016775),
    (80.814332247557, 111.72413793103448),
    (80.814332247557, 117.76328052190121),
    (66.93811074918567, 117.76328052190121),
    (59.902280130293164, 117.76328052190121),
    (52.86644951140065, 117.76328052190121),
    (46.02605863192183, 117.76328052190121),
    (38.990228013029316, 117.76328052190121),
    (73.97394136807817, 135.8807082945014),
    (66.93811074918567, 135.8807082945014),
    (59.902280130293164, 135.8807082945014),
    (52.86644951140065, 135.8807082945014),
    (46.02605863192183, 135.8807082945014),
    (73.97394136807817, 141.91985088536813),
    (66.93811074918567, 141.91985088536813),
    (59.902280130293164, 141.91985088536813),
    (73.97394136807817, 147.9589934762348),
    (66.93811074918567, 147.9589934762348),
    (59.902280130293164, 147.9589934762348),
    (73.97394136807817, 153.9981360671016),
    (80.814332247557, 141.91985088536813),
    (80.814332247557, 147.9589934762348),
    (80.814332247557, 153.9981360671016),
    (66.93811074918567, 153.9981360671016),
    (59.902280130293164, 153.9981360671016),
    (73.97394136807817, 160.03727865796827),
    (66.93811074918567, 160.03727865796827),
    (59.902280130293164, 160.03727865796827),
    (73.97394136807817, 166.07642124883506),
    (66.93811074918567, 166.07642124883506),
    (59.902280130293164, 166.07642124883506),
    (73.97394136807817, 172.1155638397018),
    (66.93811074918567, 172.1155638397018),
    (59.902280130293164, 172.1155638397018),
    (73.97394136807817, 178.15470643056852),
    (66.93811074918567, 178.15470643056852),
    (59.902280130293164, 178.15470643056852),
    (52.86644951140065, 141.91985088536813),
    (46.02605863192183, 141.91985088536813),
    (46.02605863192183, 147.9589934762348),
    (46.02605863192183, 160.03727865796827),
    (52.86644951140065, 160.03727865796827),
    (52.86644951140065, 166.07642124883506),
    (38.990228013029316, 135.8807082945014),
    (73.97394136807817, 129.84156570363467),
    (66.93811074918567, 129.84156570363467),
    (59.902280130293164, 129.84156570363467),
    (52.86644951140065, 129.84156570363467),
    (46.02605863192183, 129.84156570363467),
    (38.990228013029316, 129.84156570363467),
    (31.563517915309447, 117.76328052190121),
    (24.723127035830615, 117.76328052190121),
    (17.68729641693811, 117.76328052190121),
    (10.651465798045606, 117.76328052190121),
    (10.651465798045606, 57.204100652376525),
    (3.6156351791531023, 57.204100652376525),
    (10.651465798045606, 51.33271202236719),
    (10.651465798045606, 45.29356943150049),
    (10.651465798045606, 39.25442684063373),
    (10.651465798045606, 33.21528424976702),
    (10.651465798045606, 27.176141658900264),
    (10.651465798045606, 21.13699906803356),
    (10.651465798045606, 15.0978564771668),
    (10.651465798045606, 9.058713886300097),
    (10.651465798045606, 3.0195712954333658),
    (10.651465798045606, -3.0195712954333658),
    (10.651465798045606, -9.058713886300097),
    (10.651465798045606, -15.097856477166829),
    (3.6156351791531023, -15.097856477166829),
    (-3.420195439739416, -15.097856477166829),
    (-3.420195439739416, -9.058713886300097),
    (3.6156351791531023, -9.058713886300097),
    (3.6156351791531023, -3.0195712954333658),
    (3.6156351791531023, 3.0195712954333658),
    (3.6156351791531023, 9.058713886300097),
    (3.6156351791531023, 15.0978564771668),
    (3.6156351791531023, 21.13699906803356),
    (3.6156351791531023, 27.176141658900264),
    (3.6156351791531023, 33.21528424976702),
    (3.6156351791531023, 39.25442684063373),
    (3.6156351791531023, 45.29356943150049),
    (3.6156351791531023, 51.16495806150979),
    (-3.420195439739416, 51.16495806150979),
    (-3.420195439739416, 45.29356943150049),
    (-3.420195439739416, 39.25442684063373),
    (-3.420195439739416, 33.21528424976702),
    (-3.420195439739416, 27.176141658900264),
    (-3.420195439739416, 21.13699906803356),
    (-3.420195439739416, 15.0978564771668),
    (-3.420195439739416, 9.058713886300097),
    (-3.420195439739416, 3.0195712954333658),
    (-3.420195439739416, -3.0195712954333658),
    (-10.45602605863192, -9.058713886300097),
    (-10.45602605863192, 51.16495806150979),
    (-10.45602605863192, 45.29356943150049),
    (-10.45602605863192, 39.25442684063373),
    (-10.45602605863192, 33.21528424976702),
    (-10.45602605863192, 27.176141658900264),
    (-10.45602605863192, 21.13699906803356),
    (-10.45602605863192, 15.0978564771668),
    (-38.599348534201965, 51.16495806150979),
    (-31.563517915309447, 51.16495806150979),
    (-38.599348534201965, 45.29356943150049),
    (-45.63517915309447, 45.29356943150049),
    (-38.599348534201965, 39.25442684063373),
    (-38.599348534201965, 33.21528424976702),
    (-38.599348534201965, 27.176141658900264),
    (-38.599348534201965, 21.13699906803356),
    (-38.599348534201965, 15.0978564771668),
    (-45.63517915309447, 33.21528424976702),
    (-45.63517915309447, 27.176141658900264),
    (-45.63517915309447, 21.13699906803356),
    (-45.63517915309447, 15.0978564771668),
    (-52.671009771986974, 33.21528424976702),
    (-52.671009771986974, 27.176141658900264),
    (-52.671009771986974, 21.13699906803356),
    (-59.70684039087948, 27.176141658900264),
    (-59.70684039087948, 21.13699906803356),
    (-52.671009771986974, 15.0978564771668),
    (-10.45602605863192, 9.058713886300097),
    (-17.49185667752444, 45.29356943150049),
    (-17.49185667752444, 39.25442684063373),
    (-17.49185667752444, 33.21528424976702),
    (-17.49185667752444, 27.176141658900264),
    (-17.49185667752444, 21.13699906803356),
    (-17.49185667752444, 15.0978564771668),
    (-17.49185667752444, 9.058713886300097),
    (-24.52768729641693, 39.25442684063373),
    (-24.52768729641693, 33.21528424976702),
    (-24.52768729641693, 27.176141658900264),
    (-24.52768729641693, 21.13699906803356),
    (-24.52768729641693, 15.0978564771668),
    (-31.563517915309447, 39.25442684063373),
    (-31.563517915309447, 33.21528424976702),
    (-31.563517915309447, 27.176141658900264),
    (-31.563517915309447, 21.13699906803356),
    (-31.563517915309447, 15.0978564771668),
    (-24.52768729641693, 9.058713886300097),
    (-10.45602605863192, 3.0195712954333658),
    (-10.45602605863192, -3.0195712954333658),
    (24.723127035830615, 3.0195712954333658),
    (24.723127035830615, 9.058713886300097),
    (24.723127035830615, 9.058713886300097),
    (73.97394136807817, 21.13699906803356),
    (66.93811074918567, -21.13699906803356),
    (73.97394136807817, -21.13699906803356),
    (81.00977198697069, -21.13699906803356),
    (81.00977198697069, -27.176141658900292),
    (81.00977198697069, -33.21528424976702),
    (81.00977198697069, -39.254426840633755),
    (81.00977198697069, -69.45013979496738),
    (73.97394136807817, -69.45013979496738),
    (66.93811074918567, -69.45013979496738),
    (66.93811074918567, -63.41099720410065),
    (59.902280130293164, -63.41099720410065),
    (52.86644951140065, -57.53960857409133),
    (52.86644951140065, -63.41099720410065),
    (52.86644951140065, -69.45013979496738),
    (59.902280130293164, -69.45013979496738),
    (59.902280130293164, -75.48928238583412),
    (52.86644951140065, -75.48928238583412),
    (73.97394136807817, -153.9981360671016),
    (73.97394136807817, -160.03727865796833),
    (66.93811074918567, -141.91985088536813),
    (66.93811074918567, -147.95899347623487),
    (66.93811074918567, -153.9981360671016),
    (73.97394136807817, -166.07642124883503),
    (-59.70684039087948, -63.41099720410065),
    (-59.70684039087948, -69.45013979496738),
    (-66.74267100977201, -69.45013979496738),
    (-73.77850162866449, -75.48928238583412),
    (-80.81433224755699, -75.48928238583412),
    (-80.81433224755699, -69.45013979496738),
];
