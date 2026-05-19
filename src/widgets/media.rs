use crate::SF_PRO_DISPLAY_BOLD;
use crate::l10n::L10n;
use crate::media::MediaMetadata;
use crate::message::Message;
use crate::widgets::{ClearCache, WID_R0, WidgetId};
use chrono::{DateTime, Utc};
use iced::border::Radius;
use iced::widget::canvas::{Cache, Path};
use iced::widget::{button, canvas, column, container, row, stack, svg, text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

pub struct MediaWidget {
    pub id: WidgetId,
    pub style: MediaStyle,
}

impl Default for MediaWidget {
    fn default() -> Self {
        Self {
            id: WID_R0,
            style: MediaStyle::MediaHalf(MediaWidgetHalf::default()),
        }
    }
}

impl MediaWidget {
    pub fn view<'a>(
        &'a self,
        media_metadata: &'a Option<MediaMetadata>,
        theme: &'a Theme,
        size: Size,
        gc1: Color,
        gc2: Color,
        time: &'a DateTime<Utc>,
        seek_preview: Option<f32>,
        voluem_preview: Option<f32>,
        volume: f32,
        l10n: &'a L10n,
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
            l10n,
        )
    }
}

impl ClearCache for MediaWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

pub enum MediaStyle {
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
        time: &'a DateTime<Utc>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
        l10n: &'a L10n,
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
                l10n,
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
                l10n,
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
        time: &'a DateTime<Utc>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        let s = size.width.min(size.height);
        let palette = theme.palette();

        let thumbnail =
            if let Some(handle) = media_metadata.as_ref().and_then(|m| m.thumbnail.as_ref()) {
                container(
                    iced::widget::image(handle.clone())
                        .width(Length::Fixed(s * 0.35))
                        .height(Length::Fixed(s * 0.35))
                        .content_fit(iced::ContentFit::ScaleDown),
                )
                .width(Length::Fixed(s * 0.35))
                .height(Length::Fixed(s * 0.35))
            } else {
                let handle =
                    svg::Handle::from_memory(include_bytes!("../../icons/media-thumbnail.svg"));

                container(svg(handle).width(Length::Fixed(s)).height(Length::Fixed(s)))
                    .width(Length::Fixed(s * 0.35))
                    .height(Length::Fixed(s * 0.35))
            };

        let vol = volume_preview.unwrap_or(volume);

        let vol_icon = if vol == 0.0 {
            include_bytes!("../../icons/silent.svg").as_ref()
        } else if vol < 0.33 {
            include_bytes!("../../icons/low-volume.svg").as_ref()
        } else if vol < 0.66 {
            include_bytes!("../../icons/med-volume.svg").as_ref()
        } else {
            include_bytes!("../../icons/full-volume.svg").as_ref()
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
                None => (l10n.get("not-playing"), "—".to_string(), false, 0, 0, 0, 0),
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
        let fmt_remaining = |secs: i64| format!("-{:02}:{:02}", secs / 60, secs % 60);

        let timecode = row![
            text(fmt_time(position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
            iced::widget::Space::new().width(Length::Fill),
            text(fmt_remaining(duration - position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
        ]
        .width(Length::Fixed(s * 0.8));

        let controls = row![
            btn(
                svg::Handle::from_memory(include_bytes!("../../icons/previous.svg")),
                s * 0.12,
                Message::PreviousTrack
            ),
            if is_playing {
                btn(
                    svg::Handle::from_memory(include_bytes!("../../icons/pause.svg")),
                    s * 0.12,
                    Message::Pause,
                )
            } else {
                btn(
                    svg::Handle::from_memory(include_bytes!("../../icons/play.svg")),
                    s * 0.12,
                    Message::Play,
                )
            },
            btn(
                svg::Handle::from_memory(include_bytes!("../../icons/next.svg")),
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
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::None)
                    .color(palette.text),
                text(artist)
                    .size(s * 0.03)
                    .width(Length::Fixed(s * 0.4))
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::None)
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
pub struct MediaWidgetFull {
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
        time: &'a DateTime<Utc>,
        seek_preview: Option<f32>,
        volume_preview: Option<f32>,
        volume: f32,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        let s = size.height.min(size.width / 2.0);
        let palette = theme.palette();

        let thumbnail =
            if let Some(handle) = media_metadata.as_ref().and_then(|m| m.thumbnail.as_ref()) {
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
                let handle =
                    svg::Handle::from_memory(include_bytes!("../../icons/media-thumbnail.svg"));

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
                None => (l10n.get("not-playing"), "—".to_string(), false, 0, 0, 0, 0),
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
        let fmt_remaining = |secs: i64| format!("-{:02}:{:02}", secs / 60, secs % 60);

        let timecode = row![
            text(fmt_time(position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
            iced::widget::Space::new().width(Length::Fill),
            text(fmt_remaining(duration - position))
                .size(s * 0.03)
                .color(palette.text)
                .font(SF_PRO_DISPLAY_BOLD),
        ]
        .width(Length::Fixed(s * 0.8));

        let controls = row![
            btn(
                svg::Handle::from_memory(include_bytes!("../../icons/previous.svg")),
                s * 0.18,
                Message::PreviousTrack
            ),
            if is_playing {
                btn(
                    svg::Handle::from_memory(include_bytes!("../../icons/pause.svg")),
                    s * 0.18,
                    Message::Pause,
                )
            } else {
                btn(
                    svg::Handle::from_memory(include_bytes!("../../icons/play.svg")),
                    s * 0.18,
                    Message::Play,
                )
            },
            btn(
                svg::Handle::from_memory(include_bytes!("../../icons/next.svg")),
                s * 0.18,
                Message::NextTrack
            ),
        ]
        .spacing(s * 0.12)
        .align_y(iced::Alignment::Center);

        let vol = volume_preview.unwrap_or(volume);

        let vol_icon = if vol == 0.0 {
            include_bytes!("../../icons/silent.svg").as_ref()
        } else if vol < 0.33 {
            include_bytes!("../../icons/low-volume.svg").as_ref()
        } else if vol < 0.66 {
            include_bytes!("../../icons/med-volume.svg").as_ref()
        } else {
            include_bytes!("../../icons/full-volume.svg").as_ref()
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
                        .shaping(text::Shaping::Advanced)
                        .wrapping(text::Wrapping::None),
                    text(artist)
                        .size(s * 0.05)
                        .font(SF_PRO_DISPLAY_BOLD)
                        .color(palette.primary)
                        .width(Length::Fixed(s * 0.8))
                        .shaping(text::Shaping::Advanced)
                        .wrapping(text::Wrapping::None),
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
