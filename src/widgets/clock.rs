use crate::l10n::L10n;
use crate::message::Message;
use crate::weather::{GeoResult, WeatherStatus};
use crate::widgets::{ClearCache, WID_L0, WidgetId};
use crate::{
    SF_PRO_COMPRESSED_SEMIBOLD, SF_PRO_DISPLAY_BLACK, SF_PRO_DISPLAY_MEDIUM, SF_PRO_EXPANDED_BOLD,
    SF_PRO_ROUNDED_BLACK,
};
use chrono::*;
use chrono_tz::Tz;
use iced::border::Radius;
use iced::theme::Base;
use iced::widget::canvas::{Cache, Frame, LineCap, Path, Stroke, stroke};
use iced::widget::{
    button, canvas, column, container, mouse_area, responsive, row, stack, svg, text, text_input,
};
use iced::{
    Alignment, Color, Degrees, Element, Length, Padding, Pixels, Point, Radians, Rectangle,
    Renderer, Size, Theme, Vector, alignment, color, mouse, padding,
};
use iced_anim::{Animated, Animation, Easing};
use std::cell::Cell;
use std::f32::consts::TAU;
use std::f64::consts::PI;
use std::time::Duration;

pub struct ClockWidget {
    pub id: WidgetId,
    pub style: ClockStyle,
    pub hover: Animated<f32>,
    pub preferences_open: bool,
    pub custom_weather: Option<WeatherStatus>,
    pub city_input: String,
    pub city_results: Vec<GeoResult>,
    pub selected_city: Option<GeoResult>,
    pub world_city_inputs: [String; 4],
    pub world_city_results: [Vec<GeoResult>; 4],
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self {
            id: WID_L0,
            style: ClockStyle::AnalogueHalf(AnalogueClockHalf::default()),
            hover: Animated::new(
                0.0f32,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            preferences_open: false,
            custom_weather: None,
            city_input: String::new(),
            city_results: vec![],
            selected_city: None,
            world_city_inputs: Default::default(),
            world_city_results: Default::default(),
        }
    }
}

impl ClockWidget {
    pub fn new_with_id(id: WidgetId, style: ClockStyle) -> Self {
        Self {
            id: id,
            style: style,
            hover: Animated::new(
                0.0f32,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            preferences_open: false,
            custom_weather: None,
            city_input: String::new(),
            city_results: vec![],
            selected_city: None,
            world_city_inputs: Default::default(),
            world_city_results: Default::default(),
        }
    }

    pub fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        size: Size,
        smooth_tick: bool,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        match self.style {
            ClockStyle::DigitalCityHalf(_)
            | ClockStyle::AnalogueCityHalf(_)
            | ClockStyle::MinimalCityHalf(_)
            | ClockStyle::AnalogueRectCityHalf(_) => {
                let id = self.id;

                let sh = size.height;
                let sw = size.width;

                let primary = theme.palette().primary;

                let t_btn = *self.hover.value();
                let btn_color = Color {
                    r: primary.r * t_btn + 0.0 * (1.0 - t_btn),
                    g: primary.g * t_btn + 0.0 * (1.0 - t_btn),
                    b: primary.b * t_btn + 0.0 * (1.0 - t_btn),
                    a: 1.0,
                };

                let city_label = l10n.get("city").clone();
                let search_placeholder = l10n.get("search-city").clone();
                let preferences_label = l10n.get("preferences").clone();

                let mn = size.height.min(size.width);

                stack![
                    self.style.view(
                        time,
                        &self.selected_city,
                        weather,
                        &self.custom_weather,
                        theme,
                        size,
                        smooth_tick,
                        l10n
                    ),
                    Animation::new(
                        &self.hover,
                        container(
                            mouse_area(
                                button(
                                    svg(svg::Handle::from_memory(include_bytes!(
                                        "../../icons/brush.svg"
                                    )))
                                    .style(move |_theme: &Theme, _status| svg::Style {
                                        color: Some(btn_color),
                                        ..Default::default()
                                    })
                                    .width(Length::Fixed(sw.min(sh) * 0.1 * t_btn.max(0.3)))
                                    .height(Length::Fixed(sw.min(sh) * 0.1 * t_btn.max(0.3))),
                                )
                                .style(|_, _| button::Style {
                                    background: None,
                                    ..Default::default()
                                })
                                .on_press(Message::OpenWidgetPreferences(id))
                            )
                            .on_enter(Message::WidgetHover(id, true))
                            .on_exit(Message::WidgetHover(id, false)),
                        )
                        .padding(Padding::new(sw.min(sh) * 0.03))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Alignment::Start)
                        .align_y(Alignment::End)
                    )
                    .on_update(move |e| Message::WidgetAnimate(id, e)),
                    mouse_area(if self.preferences_open {
                        container(
                            mouse_area(
                                container(
                                    container(
                                        column![
                                            row![
                                                container(
                                                    text(preferences_label)
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
                                                    .on_press(Message::CloseWidgetPreferences(id))
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
                                                            background: Some(
                                                                iced::Background::Color(color),
                                                            ),
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
                                            row![
                                                container(
                                                    text(city_label)
                                                        .size(mn * 0.022)
                                                        .color(theme.palette().text)
                                                )
                                                .width(Length::Fill)
                                                .align_x(iced::Alignment::Start),
                                                container(column![
                                                    text_input(
                                                        search_placeholder.as_str(),
                                                        &self.city_input
                                                    )
                                                    .on_input(move |s| {
                                                        Message::WidgetCityInputChanged(id, s)
                                                    })
                                                    .width(Length::Fixed(mn * 0.2))
                                                    .style(move |_t, _status| {
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
                                                    .size(mn * 0.02),
                                                    container(column(
                                                        self.city_results
                                                            .iter()
                                                            .map(|city| {
                                                                let name = city.name.clone();
                                                                let city = city.clone();
                                                                button(text(name).size(mn * 0.02))
                                                                    .style(move |_t, _status| {
                                                                        button::Style {
                                                                            background: None,
                                                                            text_color: theme
                                                                                .palette()
                                                                                .text,
                                                                            ..Default::default()
                                                                        }
                                                                    })
                                                                    .on_press(
                                                                        Message::WidgetCitySelected(
                                                                            id, city,
                                                                        ),
                                                                    )
                                                                    .into()
                                                            })
                                                            .collect::<Vec<_>>()
                                                    ))
                                                    .style(move |_t| container::Style {
                                                        background: Some(iced::Background::Color(
                                                            Color::BLACK
                                                        )),
                                                        border: iced::Border {
                                                            color: theme.palette().primary,
                                                            width: 1.0,
                                                            radius: 4.0.into(),
                                                        },
                                                        ..Default::default()
                                                    })
                                                ])
                                                .align_x(iced::Alignment::End)
                                            ]
                                        ]
                                        .width(Length::Fill)
                                        .spacing(size.height * 0.03),
                                    )
                                    .padding(mn * 0.015)
                                    //window size
                                    .width(Length::Fixed(mn * 0.7))
                                    .height(Length::Fixed(mn * 0.4))
                                    .style(move |_| {
                                        container::Style {
                                            background: Some(iced::Background::Color(
                                                Color::from_rgb8(23, 23, 23),
                                            )),
                                            border: iced::Border {
                                                radius: (mn * 0.015).into(),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        }
                                    }),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(iced::Alignment::Center)
                                .align_y(iced::Alignment::Center),
                            )
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
            }
            ClockStyle::WorldHalf(_) => {
                let id = self.id;

                let sh = size.height;
                let sw = size.width;

                let primary = theme.palette().primary;

                let t_btn = *self.hover.value();
                let btn_color = Color {
                    r: primary.r * t_btn + 0.0 * (1.0 - t_btn),
                    g: primary.g * t_btn + 0.0 * (1.0 - t_btn),
                    b: primary.b * t_btn + 0.0 * (1.0 - t_btn),
                    a: 1.0,
                };

                let city_label = l10n.get("city").clone();
                let search_placeholder = l10n.get("search-city").clone();
                let preferences_label = l10n.get("preferences").clone();

                let mn = size.height.min(size.width);

                let make_city_row = |i: usize| {
                    let label = format!("{} {}", city_label, i + 1);
                    row![
                        container(text(label).size(mn * 0.022).color(theme.palette().text))
                            .width(Length::Fill)
                            .align_x(iced::Alignment::Start),
                        container(column![
                            text_input(search_placeholder.as_str(), &self.world_city_inputs[i])
                                .on_input(move |s| Message::WorldCityInputChanged(id, i, s))
                                .width(Length::Fixed(mn * 0.2))
                                .style(move |_t, _status| text_input::Style {
                                    value: theme.palette().text,
                                    placeholder: theme.palette().text,
                                    selection: theme.palette().primary,
                                    background: iced::Background::Color(Color::TRANSPARENT),
                                    border: iced::Border {
                                        color: theme.palette().primary,
                                        width: 1.0,
                                        radius: 4.0.into(),
                                    },
                                    icon: theme.palette().text,
                                })
                                .size(mn * 0.02),
                            container(column(
                                self.world_city_results[i]
                                    .iter()
                                    .map(|city| {
                                        let name = city.name.clone();
                                        let city = city.clone();
                                        button(text(name).size(mn * 0.02))
                                            .style(move |_t, _status| button::Style {
                                                background: None,
                                                text_color: theme.palette().text,
                                                ..Default::default()
                                            })
                                            .on_press(Message::WorldCitySelected(id, i, city))
                                            .into()
                                    })
                                    .collect::<Vec<_>>()
                            ))
                            .style(move |_t| container::Style {
                                background: Some(iced::Background::Color(Color::BLACK)),
                                border: iced::Border {
                                    color: theme.palette().primary,
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            })
                        ])
                        .align_x(iced::Alignment::End),
                    ]
                    .width(Length::Fill)
                };

                stack![
                    self.style.view(
                        time,
                        &self.selected_city,
                        weather,
                        &self.custom_weather,
                        theme,
                        size,
                        smooth_tick,
                        l10n
                    ),
                    Animation::new(
                        &self.hover,
                        container(
                            mouse_area(
                                button(
                                    svg(svg::Handle::from_memory(include_bytes!(
                                        "../../icons/brush.svg"
                                    )))
                                    .style(move |_theme: &Theme, _status| svg::Style {
                                        color: Some(btn_color),
                                        ..Default::default()
                                    })
                                    .width(Length::Fixed(sw.min(sh) * 0.1 * t_btn.max(0.3)))
                                    .height(Length::Fixed(sw.min(sh) * 0.1 * t_btn.max(0.3))),
                                )
                                .style(|_, _| button::Style {
                                    background: None,
                                    ..Default::default()
                                })
                                .on_press(Message::OpenWidgetPreferences(id))
                            )
                            .on_enter(Message::WidgetHover(id, true))
                            .on_exit(Message::WidgetHover(id, false)),
                        )
                        .padding(Padding::new(sw.min(sh) * 0.03))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Alignment::Start)
                        .align_y(Alignment::End)
                    )
                    .on_update(move |e| Message::WidgetAnimate(id, e)),
                    mouse_area(if self.preferences_open {
                        container(
                            mouse_area(
                                container(
                                    container(
                                        column![
                                            row![
                                                container(
                                                    text(preferences_label)
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
                                                    .on_press(Message::CloseWidgetPreferences(id))
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
                                                            background: Some(
                                                                iced::Background::Color(color),
                                                            ),
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
                                            make_city_row(0),
                                            make_city_row(1),
                                            make_city_row(2),
                                            make_city_row(3),
                                        ]
                                        .width(Length::Fill)
                                        .spacing(size.height * 0.03),
                                    )
                                    .padding(mn * 0.015)
                                    //window size
                                    .width(Length::Fixed(mn * 0.7))
                                    .height(Length::Fixed(mn * 0.6))
                                    .style(move |_| {
                                        container::Style {
                                            background: Some(iced::Background::Color(
                                                Color::from_rgb8(23, 23, 23),
                                            )),
                                            border: iced::Border {
                                                radius: (mn * 0.015).into(),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        }
                                    }),
                                )
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(iced::Alignment::Center)
                                .align_y(iced::Alignment::Center),
                            )
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
            }
            _ => self.style.view(
                time,
                &self.selected_city,
                weather,
                &self.custom_weather,
                theme,
                size,
                smooth_tick,
                l10n,
            ),
        }
    }
}

impl ClearCache for ClockWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

pub enum ClockStyle {
    DigitalHalf(DigitalClockHalf),
    DigitalCityHalf(DigitalClockCityHalf),
    AnalogueHalf(AnalogueClockHalf),
    AnalogueCityHalf(AnalogueClockCityHalf),
    MinimalHalf(MinimalClockHalf),
    MinimalCityHalf(MinimalClockCityHalf),
    AnalogueRectHalf(AnalogueRectClockHalf),
    AnalogueRectCityHalf(AnalogueRectClockCityHalf),
    AnalogueRectFull(AnalogueRectClockFull),
    WorldHalf(WorldClockHalf),
    WorldFull(WorldClockFull),
}

impl ClockStyle {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        weather: &'a WeatherStatus,
        custom_weather: &'a Option<WeatherStatus>,
        theme: &'a Theme,
        size: Size,
        smooth_tick: bool,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        let effective_weather = custom_weather.as_ref().unwrap_or(weather);

        match self {
            ClockStyle::DigitalHalf(clock) => clock.view(time, tz),
            ClockStyle::DigitalCityHalf(clock) => {
                clock.view(time, tz, effective_weather, size, theme)
            }
            ClockStyle::AnalogueHalf(clock) => clock.view(time, tz, smooth_tick),
            ClockStyle::AnalogueCityHalf(clock) => {
                clock.view(time, tz, effective_weather, smooth_tick, size, theme)
            }
            ClockStyle::MinimalHalf(clock) => clock.view(time, tz, smooth_tick),
            ClockStyle::MinimalCityHalf(clock) => {
                clock.view(time, tz, effective_weather, smooth_tick, size, theme)
            }
            ClockStyle::AnalogueRectHalf(clock) => clock.view(time, tz, smooth_tick),
            ClockStyle::AnalogueRectCityHalf(clock) => {
                clock.view(time, tz, effective_weather, smooth_tick, size, theme)
            }
            ClockStyle::AnalogueRectFull(clock) => clock.view(time, tz, smooth_tick, l10n),
            ClockStyle::WorldHalf(clock) => clock.view(time, smooth_tick, theme),
            ClockStyle::WorldFull(clock) => clock.view(time, weather, theme, size, l10n),
        }
    }
}

impl ClearCache for ClockStyle {
    fn clear_cache(&self) {
        match self {
            ClockStyle::AnalogueHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueCityHalf(clock) => clock.clear_cache(),
            ClockStyle::MinimalHalf(clock) => clock.clear_cache(),
            ClockStyle::MinimalCityHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueRectHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueRectCityHalf(clock) => clock.clear_cache(),
            ClockStyle::AnalogueRectFull(clock) => clock.clear_cache(),
            ClockStyle::WorldFull(clock) => clock.clear_cache(),
            _ => {}
        }
    }
}

#[derive(Default)]
pub struct DigitalClockHalf {
    cache: Cache,
}

impl DigitalClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
    ) -> Element<'a, Message> {
        self.cache.clear();
        canvas((self, time, tz))
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message>
    for (
        &'a DigitalClockHalf,
        &'a DateTime<Utc>,
        &'a Option<GeoResult>,
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
        let (widget, now, selected_city) = self;

        let now = if let Some(city) = selected_city {
            if let Ok(tz) = city.timezone.parse::<Tz>() {
                now.with_timezone(&tz).fixed_offset()
            } else {
                now.with_timezone(&Local).fixed_offset()
            }
        } else {
            now.with_timezone(&Local).fixed_offset()
        };

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

            let font_size = s * 0.45;

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
pub struct DigitalClockCityHalf {
    clock_frame: DigitalClockHalf,
}

impl DigitalClockCityHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        weather: &'a WeatherStatus,
        size: Size,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let wdt = size.width;
        let hgh = size.height;
        let scale = (wdt / 960.0).min(hgh / 1080.0);

        let (city_label, temp_label) = match weather {
            WeatherStatus::Ok(w) => (
                container(
                    text(format!("{:.3}", w.city.as_ref().unwrap()).to_uppercase())
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 1050.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text(format!(
                        "{:.0}",
                        if w.current.as_ref().unwrap().apparent_temperature.abs() < 1.0 {
                            0.0
                        } else {
                            w.current.as_ref().unwrap().apparent_temperature
                        }
                    ))
                    .size(scale * 65.0)
                    .color(theme.palette().primary)
                    .font(SF_PRO_ROUNDED_BLACK)
                    .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 + 50.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
            _ => (
                container(
                    text("n/a")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 1050.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text("-")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .font(SF_PRO_ROUNDED_BLACK)
                        .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 + 50.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
        };

        stack![
            city_label,
            temp_label,
            stack![self.clock_frame.view(time, tz)],
        ]
        .width(Length::Fill)
        .into()
    }
}

impl ClearCache for DigitalClockCityHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
pub struct AnalogueClockHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueHalf,
}

impl AnalogueClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
    ) -> Element<'a, Message> {
        stack![
            self.clock_frame.view(),
            self.hands.view(time, tz, smooth_tick, false)
        ]
        .into()
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
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
        adaptive: bool,
    ) -> Element<'a, Message> {
        self.cache.clear();

        canvas((self, time, tz, smooth_tick, adaptive))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message>
    for (
        &'a Hands,
        &'a DateTime<Utc>,
        &'a Option<GeoResult>,
        bool,
        bool,
    )
{
    type State = HandsAnimState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.palette();
        let (widget, now, selected_city, smooth_tick, adaptive) = self;

        let now = if let Some(city) = selected_city {
            if let Ok(tz) = city.timezone.parse::<Tz>() {
                now.with_timezone(&tz).fixed_offset()
            } else {
                now.with_timezone(&Local).fixed_offset()
            }
        } else {
            now.with_timezone(&Local).fixed_offset()
        };

        let black = if *adaptive == true {
            let hour = now.hour();
            hour >= 7 && hour < 20
        } else {
            false
        };

        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();
            let radius = frame.width().min(frame.height()) / 2.3;
            let neck_color = if theme.name() == "red_dark" || !black {
                palette.text
            } else {
                palette.background
            };

            let seconds = if *smooth_tick {
                now.second() as f32 + now.nanosecond() as f32 / 1_000_000_000.0
            } else {
                now.second() as f32
            };

            let target_hour = (Radians::from(hand_rotation(now.hour(), 12))
                + Radians::from(hand_rotation(now.minute(), 60)) / 12.0)
                .0;
            let target_min =
                Radians::from(hand_rotation(now.minute() * 15 + now.second() / 4, 900)).0;
            let target_sec = hand_rotation_sec(seconds, 60.0).0;

            _state
                .hour
                .set(lerp_angle(_state.hour.get(), target_hour, 0.10));
            _state
                .minute
                .set(lerp_angle(_state.minute.get(), target_min, 0.15));
            _state
                .second
                .set(lerp_angle(_state.second.get(), target_sec, 0.25));

            let hour_hand_angle = _state.hour.get();
            let minute_angle = _state.minute.get();
            let second_angle = _state.second.get();

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
                    f.translate(Vector::new(0.5, 0.5));
                    let shadow = Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.4,
                    };

                    f.stroke(
                        &hour_neck,
                        Stroke {
                            width: hour_neck_width * 1.2,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );

                    f.stroke(
                        &hour_body,
                        Stroke {
                            width: hour_body_width * 1.2,
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
                        style: stroke::Style::Solid(neck_color),
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &hour_neck,
                    Stroke {
                        width: hour_neck_width,
                        style: stroke::Style::Solid(neck_color),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &hour_body,
                    Stroke {
                        width: hour_body_width,
                        style: stroke::Style::Solid(neck_color),
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
                            width: min_neck_width * 1.2,
                            style: stroke::Style::Solid(shadow),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );

                    f.stroke(
                        &min_body,
                        Stroke {
                            width: min_body_width * 1.2,
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
                        style: stroke::Style::Solid(neck_color),
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &min_neck,
                    Stroke {
                        width: min_neck_width,
                        style: stroke::Style::Solid(neck_color),
                        line_cap: LineCap::Round,
                        ..Stroke::default()
                    },
                );

                frame.stroke(
                    &min_body,
                    Stroke {
                        width: min_body_width,
                        style: stroke::Style::Solid(neck_color),
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
struct HandsAnimState {
    hour: Cell<f32>,
    minute: Cell<f32>,
    second: Cell<f32>,
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
                    font: SF_PRO_DISPLAY_MEDIUM,
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
pub struct AnalogueClockCityHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueHalf,
}

impl AnalogueClockCityHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        weather: &'a WeatherStatus,
        smooth_tick: bool,
        size: Size,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let wdt = size.width;
        let hgh = size.height;
        let scale = (wdt / 960.0).min(hgh / 1080.0);

        let (city_label, temp_label) = match weather {
            WeatherStatus::Ok(w) => (
                container(
                    text(format!("{:.3}", w.city.as_ref().unwrap()).to_uppercase())
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 800.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text(format!(
                        "{:.0}",
                        if w.current.as_ref().unwrap().apparent_temperature.abs() < 1.0 {
                            0.0
                        } else {
                            w.current.as_ref().unwrap().apparent_temperature
                        }
                    ))
                    .size(scale * 65.0)
                    .color(theme.palette().primary)
                    .font(SF_PRO_ROUNDED_BLACK)
                    .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 200.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
            _ => (
                container(
                    text("n/a")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 800.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text("-")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .font(SF_PRO_ROUNDED_BLACK)
                        .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 200.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
        };

        stack![
            city_label,
            temp_label,
            stack![
                self.clock_frame.view(),
                self.hands.view(time, tz, smooth_tick, false)
            ],
        ]
        .width(Length::Fill)
        .into()
    }
}

impl ClearCache for AnalogueClockCityHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
pub struct MinimalClockHalf {
    hands: Hands,
    clock_frame: ClockFrameMinimalHalf,
}

impl MinimalClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
    ) -> Element<'a, Message> {
        stack![
            self.clock_frame.view(),
            self.hands.view(time, tz, smooth_tick, false)
        ]
        .into()
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
pub struct MinimalClockCityHalf {
    hands: Hands,
    clock_frame: ClockFrameMinimalHalf,
}

impl MinimalClockCityHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        weather: &'a WeatherStatus,
        smooth_tick: bool,
        size: Size,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let wdt = size.width;
        let hgh = size.height;
        let scale = (wdt / 960.0).min(hgh / 1080.0);

        let (city_label, temp_label) = match weather {
            WeatherStatus::Ok(w) => (
                container(
                    text(format!("{:.3}", w.city.as_ref().unwrap()).to_uppercase())
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 800.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text(format!(
                        "{:.0}",
                        if w.current.as_ref().unwrap().apparent_temperature.abs() < 1.0 {
                            0.0
                        } else {
                            w.current.as_ref().unwrap().apparent_temperature
                        }
                    ))
                    .size(scale * 65.0)
                    .color(theme.palette().primary)
                    .font(SF_PRO_ROUNDED_BLACK)
                    .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 200.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
            _ => (
                container(
                    text("n/a")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 800.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text("-")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .font(SF_PRO_ROUNDED_BLACK)
                        .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 200.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
        };

        stack![
            city_label,
            temp_label,
            stack![
                self.clock_frame.view(),
                self.hands.view(time, tz, smooth_tick, false)
            ],
        ]
        .width(Length::Fill)
        .into()
    }
}

impl ClearCache for MinimalClockCityHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
pub struct AnalogueRectClockHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueRectHalf,
}

impl AnalogueRectClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
    ) -> Element<'a, Message> {
        stack![
            self.clock_frame.view(),
            self.hands.view(time, tz, smooth_tick, false)
        ]
        .into()
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
                        Point::new(point.x, point.y + inner_padding_min * 3.0)
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
                        Point::new(point.x, point.y - inner_padding_min * 3.0)
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
                        Point::new(point.x + inner_padding_min * 3.0, point.y)
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
                        Point::new(point.x - inner_padding_min * 3.0, point.y)
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
                        Point::new(center.x, offset_y + inner_padding_hour * 1.8),
                    ),
                    (
                        "3",
                        Point::new(offset_x + size - inner_padding_hour * 1.75, center.y),
                    ),
                    (
                        "6",
                        Point::new(center.x, offset_y + size - inner_padding_hour * 1.8),
                    ),
                    (
                        "9",
                        Point::new(offset_x + inner_padding_hour * 1.75, center.y),
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
                        font: SF_PRO_DISPLAY_MEDIUM,
                        ..canvas::Text::default()
                    });
                }
            })
        });
        vec![static_layer]
    }
}

#[derive(Default)]
pub struct AnalogueRectClockCityHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueRectHalf,
}

impl AnalogueRectClockCityHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        weather: &'a WeatherStatus,
        smooth_tick: bool,
        size: Size,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let wdt = size.width;
        let hgh = size.height;
        let scale = (wdt / 960.0).min(hgh / 1080.0);

        let (city_label, temp_label) = match weather {
            WeatherStatus::Ok(w) => (
                container(
                    text(format!("{:.3}", w.city.as_ref().unwrap()).to_uppercase())
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 750.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text(format!(
                        "{:.0}",
                        if w.current.as_ref().unwrap().apparent_temperature.abs() < 1.0 {
                            0.0
                        } else {
                            w.current.as_ref().unwrap().apparent_temperature
                        }
                    ))
                    .size(scale * 65.0)
                    .color(theme.palette().primary)
                    .font(SF_PRO_ROUNDED_BLACK)
                    .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 250.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
            _ => (
                container(
                    text("n/a")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .height(Length::Fixed(scale * 65.0))
                        .font(SF_PRO_ROUNDED_BLACK),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 750.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
                container(
                    text("-")
                        .size(scale * 65.0)
                        .color(theme.palette().primary)
                        .font(SF_PRO_ROUNDED_BLACK)
                        .height(Length::Fixed(scale * 65.0)),
                )
                .padding(padding::top(hgh.min(wdt) / 2.0 - 250.0 * scale))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .align_x(Alignment::Center),
            ),
        };

        stack![
            city_label,
            temp_label,
            stack![
                self.clock_frame.view(),
                self.hands.view(time, tz, smooth_tick, false)
            ],
        ]
        .width(Length::Fill)
        .into()
    }
}

impl ClearCache for AnalogueRectClockCityHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
pub struct AnalogueRectClockFull {
    hands: Hands,
    clock_frame: ClockFrameAnalogueRectFull,
}

impl AnalogueRectClockFull {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        stack![
            self.clock_frame.view(time, l10n),
            self.hands.view(time, tz, smooth_tick, false)
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
    fn view<'a>(&'a self, time: &'a DateTime<Utc>, l10n: &'a L10n) -> Element<'a, Message> {
        if time.day() != self.last_day.get() {
            self.last_day.set(time.day());
            self.cache.clear();
        }

        canvas((self, time, l10n))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message>
    for (&'a ClockFrameAnalogueRectFull, &'a DateTime<Utc>, &'a L10n)
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
        let (widget, time, l10n) = self;
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
                    content: l10n
                        .get(&format!("weekday-{}", time.weekday().number_from_monday()))
                        .to_uppercase(),
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

pub struct WorldClockHalf {
    pub clock: [AdaptiveZoneClockHalf; 4],
    pub tzs: [Option<GeoResult>; 4],
}

impl WorldClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        smooth_tick: bool,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        responsive(move |size| {
            let side = size.width.min(size.height);
            let cell = side / 2.0;

            let c0 = container(self.clock[0].view(time, &self.tzs[0], smooth_tick, size, theme))
                .width(Length::Fixed(cell))
                .height(Length::Fixed(cell))
                .padding(15);

            let c1 = container(self.clock[1].view(time, &self.tzs[1], smooth_tick, size, theme))
                .width(Length::Fixed(cell))
                .height(Length::Fixed(cell))
                .padding(15);

            let c2 = container(self.clock[2].view(time, &self.tzs[2], smooth_tick, size, theme))
                .width(Length::Fixed(cell))
                .height(Length::Fixed(cell))
                .padding(15);

            let c3 = container(self.clock[3].view(time, &self.tzs[3], smooth_tick, size, theme))
                .width(Length::Fixed(cell))
                .height(Length::Fixed(cell))
                .padding(15);

            container(column![row![c0, c1], row![c2, c3]])
                .width(Length::Fixed(side))
                .height(Length::Fixed(side))
                .center(Length::Fill)
                .into()
        })
        .into()
    }
}

impl Default for WorldClockHalf {
    fn default() -> Self {
        Self {
            clock: std::array::from_fn(|_| AdaptiveZoneClockHalf::default()),
            tzs: [
                Some(GeoResult {
                    name: String::from("New York"),
                    latitude: 40.7128,
                    longitude: -74.0060,
                    timezone: String::from("America/New_York"),
                }),
                Some(GeoResult {
                    name: String::from("London"),
                    latitude: 51.5074,
                    longitude: -0.1278,
                    timezone: String::from("Europe/London"),
                }),
                Some(GeoResult {
                    name: String::from("Dubai"),
                    latitude: 25.2048,
                    longitude: 55.2708,
                    timezone: String::from("Asia/Dubai"),
                }),
                Some(GeoResult {
                    name: String::from("Tokyo"),
                    latitude: 35.6762,
                    longitude: 139.6503,
                    timezone: String::from("Asia/Tokyo"),
                }),
            ],
        }
    }
}

#[derive(Default)]
pub struct AdaptiveZoneClockHalf {
    clock_frame: ClockFrameAdaptiveZone,
    hands: Hands,
}

impl AdaptiveZoneClockHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        smooth_tick: bool,
        size: Size,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        let wdt = size.width;
        let hgh = size.height;
        let scale = (wdt / 960.0).min(hgh / 1080.0);

        let city_name = tz
            .as_ref()
            .map(|g| g.name.to_uppercase())
            .unwrap_or_else(|| String::from("n/a"));

        let city_label = container(
            text(format!("{:.3}", city_name))
                .size(scale * 40.0)
                .color(theme.palette().primary)
                .height(Length::Fixed(scale * 65.0))
                .font(SF_PRO_ROUNDED_BLACK),
        )
        .padding(padding::top(hgh.min(wdt) / 2.0 - 640.0 * scale))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center);

        stack![
            self.clock_frame.view(time, tz, theme),
            city_label,
            self.hands.view(time, tz, smooth_tick, true)
        ]
        .into()
    }
}

impl ClearCache for AdaptiveZoneClockHalf {
    fn clear_cache(&self) {
        self.clock_frame.cache.clear();
    }
}

#[derive(Default)]
pub struct ClockFrameAdaptiveZone {
    minute: Cell<u32>,
    is_dark: Cell<bool>,
    cache: Cache,
}

impl ClockFrameAdaptiveZone {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        tz: &'a Option<GeoResult>,
        theme: &'a Theme,
    ) -> Element<'a, Message> {
        if time.minute() != self.minute.get() {
            self.minute.set(time.minute());
            self.cache.clear();
        };

        canvas((self, time, tz, theme))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message>
    for (
        &'a ClockFrameAdaptiveZone,
        &'a DateTime<Utc>,
        &'a Option<GeoResult>,
        &'a Theme,
    )
{
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, now, selected_city, theme) = self;
        let palette = theme.palette();

        let now = if let Some(city) = selected_city {
            if let Ok(tz) = city.timezone.parse::<Tz>() {
                now.with_timezone(&tz).fixed_offset()
            } else {
                now.with_timezone(&Local).fixed_offset()
            }
        } else {
            now.with_timezone(&Local).fixed_offset()
        };

        if widget.is_dark.get() != (theme.name() != "classic") {
            widget.cache.clear();
        }

        let static_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();

            frame.translate(Vector::new(center.x, center.y));

            let radius = frame.width().min(frame.height()) / 2.3;

            frame.fill(
                &Path::circle(Point::ORIGIN, radius * 1.05),
                if theme.name() == "classic" {
                    if (now.hour() < 7) || (now.hour() > 20) {
                        Color::from_rgb8(37, 37, 37)
                    } else {
                        Color::from_rgb8(205, 205, 205)
                    }
                } else {
                    palette.background
                },
            );

            for hour in 1..=12 {
                let angle = Radians::from(hand_rotation(hour, 12)) - Radians::from(Degrees(90.0));

                let x = radius * angle.0.cos();
                let y = radius * angle.0.sin();

                frame.fill_text(canvas::Text {
                    content: format!("{hour}"),
                    size: (radius / 4.5).into(),
                    position: Point::new(x * 0.85, y * 0.85),
                    color: if theme.name() == "classic" {
                        if (now.hour() < 7) || (now.hour() > 20) {
                            palette.text
                        } else {
                            Color::BLACK
                        }
                    } else {
                        palette.text
                    },
                    align_x: text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    font: SF_PRO_DISPLAY_MEDIUM,
                    ..canvas::Text::default()
                });
            }
        });

        vec![static_layer]
    }
}

#[derive(Default)]
pub struct WorldClockFull {
    minute: Cell<u32>,
    cache: Cache,
}

impl WorldClockFull {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        weather: &'a WeatherStatus,
        theme: &'a Theme,
        size: Size,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        if time.minute() != self.minute.get() {
            self.minute.set(time.minute());
            self.cache.clear();
        }

        let map = svg(svg::Handle::from_memory(include_bytes!(
            "../../icons/world-map.svg"
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
            canvas((self, l10n, time, weather))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .into()
    }
}

impl<'a> canvas::Program<Message>
    for (
        &'a WorldClockFull,
        &'a L10n,
        &'a DateTime<Utc>,
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
        let (widget, l10n, time, weather) = self;
        let palette = theme.palette();

        let time = time.with_timezone(&Local);

        let static_layer = match weather {
            WeatherStatus::Ok(w) => widget.cache.draw(renderer, bounds.size(), |frame| {
                let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);

                frame.with_save(|frame| {
                    let city = w.city.as_ref().unwrap();
                    let (lat, lon) = w.coordinate.as_ref().unwrap();

                    let map_width = frame.width() * 0.85;
                    let map_height = map_width * (921.0 / 2146.0);

                    let map_offset_y = (frame.height() - map_height * 1.15) / 2.0;

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
                        content: format!("{:02}:{:02}", time.hour(), time.minute()),
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
                    content: l10n.get("location-unavailable"),
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
                        theme.palette().danger
                    } else {
                        continue;
                    };

                    let p = to_canvas(*lat, *lon);

                    let scale = (frame.width() + frame.height()) / (1920.0 + 1080.0);
                    let circle = Path::circle(p, scale * 10.0);
                    frame.fill(&circle, color);
                }

                draw_night_overlay(&mut frame, map_bounds, theme, sub_lat, sub_lon);
            }

            frame.into_geometry()
        };

        vec![night_layer, static_layer]
    }
}

impl ClearCache for WorldClockFull {
    fn clear_cache(&self) {
        self.cache.clear();
    }
}

fn hand_rotation(n: u32, total: u32) -> Degrees {
    let turns = n as f32 / total as f32;

    Degrees(360.0 * turns)
}

fn hand_rotation_sec(value: f32, max: f32) -> Radians {
    Radians(value / max * std::f32::consts::TAU)
}

fn lerp_angle(current: f32, target: f32, t: f32) -> f32 {
    let current = current.rem_euclid(TAU);
    let target = target.rem_euclid(TAU);
    let diff = ((target - current + TAU * 1.5) % TAU) - TAU / 2.0;
    current + diff * t
}

fn lat_lon_to_xy(lat: f64, lon: f64, width: f32, height: f32) -> Point {
    let x = (lon + 180.0) / 360.0 * width as f64;

    let lat_rad = lat.to_radians();
    let merc = (lat_rad.tan() + 1.0 / lat_rad.cos()).ln();
    let y = (1.0 - merc / std::f64::consts::PI) / 2.0 * height as f64;

    Point::new(x as f32, y as f32)
}

fn subsolar_point() -> (f64, f64) {
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

fn terminator_points(sub_lat_deg: f64, sub_lon_deg: f64, n: usize) -> Vec<(f64, f64)> {
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
fn draw_night_overlay(
    frame: &mut Frame,
    bounds: Rectangle,
    theme: &Theme,
    sub_lat: f64,
    sub_lon: f64,
) {
    let theme_color = theme.palette().primary;
    let color = |t: f32| -> Color {
        Color::from_rgba(theme_color.r, theme_color.g, theme_color.b, 0.3 + t * 0.5)
    };

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

            frame.stroke(
                &Path::new(|b| {
                    b.move_to(p0);
                    b.line_to(p1);
                }),
                canvas::Stroke::default()
                    .with_color(color(t))
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
                .with_color(color(t))
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
