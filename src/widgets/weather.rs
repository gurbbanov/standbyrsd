use crate::l10n::L10n;
use crate::message::Message;
use crate::settings::SpeedUnit;
use crate::weather::{GeoResult, WeatherStatus};
use crate::widgets::calendar::weekday_to_number;
use crate::widgets::{ClearCache, WID_R3, WidgetId};
use crate::{SF_PRO_DISPLAY_BLACK, SF_PRO_DISPLAY_BOLD};
use chrono::*;
use iced::theme::Base;
use iced::widget::canvas::Cache;
use iced::widget::{
    button, canvas, column, container, mouse_area, row, stack, svg, text, text_input,
};
use iced::{
    Alignment, Color, Element, Length, Padding, Pixels, Point, Rectangle, Renderer, Size, Theme,
    alignment, mouse, padding,
};
use iced_anim::{Animated, Animation, Easing};
use std::cell::Cell;
use std::time::Duration;

pub struct WeatherWidget {
    pub id: WidgetId,
    pub style: WeatherStyle,
    pub hover: Animated<f32>,
    pub preferences_open: bool,
    pub custom_weather: Option<WeatherStatus>,
    pub city_input: String,
    pub city_results: Vec<GeoResult>,
    pub selected_city: Option<GeoResult>,
}

impl Default for WeatherWidget {
    fn default() -> Self {
        Self {
            id: WID_R3,
            style: WeatherStyle::MinimalHalf(MinimalForecastHalf::default()),
            hover: Animated::new(
                0.0f32,
                Easing::EASE.with_duration(Duration::from_millis(1500)),
            ),
            preferences_open: false,
            custom_weather: None,
            city_input: String::new(),
            city_results: vec![],
            selected_city: None,
        }
    }
}

impl WeatherWidget {
    pub fn new_with_id(id: WidgetId, style: WeatherStyle) -> Self {
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
        }
    }

    pub fn view<'a>(
        &'a self,
        theme: &'a Theme,
        time: &'a DateTime<Utc>,
        weather: &'a WeatherStatus,
        size: Size,
        l10n: &'a L10n,
        speed_unit: &'a SpeedUnit,
    ) -> Element<'a, Message> {
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
                theme,
                weather,
                &self.custom_weather,
                size,
                l10n,
                speed_unit
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
                .align_x(Alignment::End)
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
                                            .style(
                                                |_, status| {
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
                                                }
                                            )
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
                                                            .on_press(Message::WidgetCitySelected(
                                                                id, city,
                                                            ))
                                                            .into()
                                                    })
                                                    .collect::<Vec<_>>()
                                            ))
                                            .style(
                                                move |_t| container::Style {
                                                    background: Some(iced::Background::Color(
                                                        Color::BLACK
                                                    )),
                                                    border: iced::Border {
                                                        color: theme.palette().primary,
                                                        width: 1.0,
                                                        radius: 4.0.into(),
                                                    },
                                                    ..Default::default()
                                                }
                                            )
                                        ])
                                        .align_x(iced::Alignment::End)
                                    ]
                                ]
                                .width(Length::Fill)
                                .spacing(size.height * 0.03),
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
                        .align_y(iced::Alignment::Center),
                        // .into(),
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
}

impl ClearCache for WeatherWidget {
    fn clear_cache(&self) {
        self.style.clear_cache();
    }
}

pub enum WeatherStyle {
    MinimalHalf(MinimalForecastHalf),
    DetailedHalf(DetailedForecastHalf),
    DailyHalf(DailyForecastHalf),
}

impl WeatherStyle {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Utc>,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        custom_weather: &'a Option<WeatherStatus>,
        size: Size,
        l10n: &'a L10n,
        speed_unit: &'a SpeedUnit,
    ) -> Element<'a, Message> {
        let effective_weather = custom_weather.as_ref().unwrap_or(weather);

        match self {
            Self::MinimalHalf(w) => w.view(theme, effective_weather, size, l10n),
            Self::DetailedHalf(w) => w.view(theme, effective_weather, size, l10n, speed_unit),
            Self::DailyHalf(w) => w.view(theme, time, effective_weather, size, l10n),
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
pub struct MinimalForecastHalf {
    cache: Cache,
}

impl MinimalForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        size: Size,
        l10n: &'a L10n,
    ) -> Element<'a, Message> {
        let w = size.width;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 110.0 * scale;
        let icon_x = w * 0.05;
        let icon_y = h / 2.0 + 270.0 * scale - icon_size - 20.0 * scale;

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
                } else if (code == 2) && (w_data.current.as_ref().unwrap().is_day == 0) {
                    svg(svg::Handle::from_memory(wmo_code_svg(101)))
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
                } else if ((51..=65).contains(&code))
                    && w_data.current.as_ref().unwrap().is_day == 0
                {
                    svg(svg::Handle::from_memory(wmo_code_svg(102)))
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
            canvas((self, l10n, weather))
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

impl<'a> canvas::Program<Message> for (&'a MinimalForecastHalf, &'a L10n, &'a WeatherStatus) {
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
        let (widget, l10n, weather) = self;

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
                        content: format!("{:.0}°", if current.temperature_2m.abs() < 1.0 { 0.0 } else { current.temperature_2m }),
                        size: Pixels(w.min(h) * 0.37),
                        position: Point::new(w * 0.05, frame.center().y + 50.0 * scale.min(h / 1080.0)),
                        color: palette.text,
                        align_y: alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}", wmo_code_description(current.weather_code, l10n)),
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
                            "{}:{:.0}° {}:{:.0}°",
                            l10n.get("high-short"),
                            if daily.apparent_temperature_max[0].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_max[0] },
                            l10n.get("low-short"),
                            if daily.apparent_temperature_min[0].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_min[0] }
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
                    content: l10n.get("weather-unavailable"),
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
pub struct DetailedForecastHalf {
    cache: Cache,
}

impl DetailedForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        weather: &'a WeatherStatus,
        size: Size,
        l10n: &'a L10n,
        speed_unit: &'a SpeedUnit,
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
            canvas((self, l10n, weather, speed_unit))
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

impl<'a> canvas::Program<Message>
    for (
        &'a DetailedForecastHalf,
        &'a L10n,
        &'a WeatherStatus,
        &'a SpeedUnit,
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
        let (widget, l10n, weather, speed_unit) = self;
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
                        content: format!("{:.0}°", if current.temperature_2m.abs() < 1.0 { 0.0 } else { current.temperature_2m }),
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
                        content: format!("↑{:.0}°", if daily.apparent_temperature_max[0].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_max[0] }),
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
                        content: format!("↓{:.0}°", if daily.apparent_temperature_min[0].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_min[0] }),
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
                        content: l10n.get("precipitation"),
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
                            .map_or(l10n.get("none-for-7d"), |(i, &v)| {
                                l10n.get_args("rain-forecast", &[
                                    ("v", v.to_string().as_str()),
                                    ("i", i.to_string().as_str()),
                                ])
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
                        content: l10n.get("wind"),
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
                        content: format!("{} {}", current.wind_speed_10m, speed_unit),
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
                        content: l10n.get("uvi"),
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
                        content: l10n.get("feels-like"),
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
                    content: l10n.get("weather-unavailable"),
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
pub struct DailyForecastHalf {
    last_day: Cell<u32>,
    cache: Cache,
}

impl DailyForecastHalf {
    fn view<'a>(
        &'a self,
        theme: &'a Theme,
        time: &'a DateTime<Utc>,
        weather: &'a WeatherStatus,
        size: Size,
        l10n: &'a L10n,
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
            canvas((self, time, l10n, weather))
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
        &'a DateTime<Utc>,
        &'a L10n,
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
        let (widget, time, l10n, weather) = self;
        let palette = theme.palette();

        let weekdays: Vec<String> = (1..=7)
            .map(|i| l10n.get(&format!("weekday-{}", i)))
            .collect();
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
                        content: format!("{:.0}°", if current.temperature_2m.abs() < 1.0 { 0.0 } else { current.temperature_2m }),
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
                        content: format!("↑{:.0}°", if daily.apparent_temperature_max[0].abs() < 1.0 { 0.0 } else {daily.apparent_temperature_max[0]}),
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
                        content: format!("↓{:.0}°", if daily.apparent_temperature_min[0].abs() < 1.0 { 0.0 } else {daily.apparent_temperature_min[0]}),
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
                            content: format!("{:.0}°", if daily.apparent_temperature_min[counter].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_min[counter] }),
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
                            content: format!("{:.0}°", if daily.apparent_temperature_max[counter].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_max[counter] }),
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
                                content: format!("{:.0}°", if daily.apparent_temperature_min[counter].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_min[counter] }),
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
                                content: format!("{:.0}°", if daily.apparent_temperature_max[counter].abs() < 1.0 { 0.0 } else { daily.apparent_temperature_max[counter ]}),
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
                    content: l10n.get("weather-unavailable"),
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

fn wmo_code_description(code: u8, l10n: &L10n) -> String {
    match code {
        0 => l10n.get("clear"),
        1 => l10n.get("mostly-clear"),
        2 => l10n.get("partly-cloudy"),
        3 => l10n.get("cloudy"),
        45..=48 => l10n.get("fog"),
        51..=55 => l10n.get("drizzle"),
        56..=57 => l10n.get("freezing-drizzle"),
        61..=63 => l10n.get("rain"),
        65 => l10n.get("heavy-rain"),
        66..=67 => l10n.get("freezing-rain"),
        71..=73 => l10n.get("snow"),
        75 => l10n.get("heavy-snow"),
        77 => l10n.get("blizzard"),
        80..=86 => l10n.get("wintry-mix"),
        95..=99 => l10n.get("thunderstorm"),
        _ => l10n.get("n-a"),
    }
}

fn wmo_code_svg(code: u8) -> &'static [u8] {
    match code {
        0 | 1 => include_bytes!("../../icons/clear.svg"),
        2 => include_bytes!("../../icons/partly-cloudy.svg"),
        3 => include_bytes!("../../icons/cloudy.svg"),
        45..=48 => include_bytes!("../../icons/fog.svg"),
        51..=57 => include_bytes!("../../icons/drizzle.svg"),
        61..=63 => include_bytes!("../../icons/rain.svg"),
        65 => include_bytes!("../../icons/heavy-rain.svg"),
        66..=67 => include_bytes!("../../icons/freezing-rain.svg"),
        71..=73 => include_bytes!("../../icons/snow.svg"),
        75 | 77 => include_bytes!("../../icons/heavy-snow.svg"),
        80..=86 => include_bytes!("../../icons/freezing-rain.svg"),
        95..=99 => include_bytes!("../../icons/thunderstorm.svg"),
        100 => include_bytes!("../../icons/clear-night.svg"),
        101 => include_bytes!("../../icons/partly-cloudy-night.svg"),
        102 => include_bytes!("../../icons/drizzle-night.svg"),
        _ => include_bytes!("../../icons/warning.svg"),
    }
}
