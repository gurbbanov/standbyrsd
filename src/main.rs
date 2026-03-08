use chrono::prelude::*;
use iced::font::{Family, Weight};
use iced::time::{self, milliseconds};
use iced::widget::canvas::{Cache, LineCap, Path, Stroke, stroke};
use iced::widget::{
    Grid, button, canvas, center, column, container, responsive, row, scrollable, stack, text,
};
use iced::window::{self, Id};
use iced::{
    Alignment, Color, Degrees, Element, Font, Length, Point, Radians, Renderer, Settings, Size,
    Subscription, Task, Theme, Vector, color,
};

pub fn main() -> iced::Result {
    iced::daemon(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .settings(Settings {
            fonts: vec![include_bytes!("../fonts/sfpro.ttf").into()],
            default_font: Font {
                family: Family::Name("SF Pro Rounded"),
                weight: Weight::Black,
                ..Font::DEFAULT
            },
            ..Settings::default()
        })
        .theme(Theme::Moonfly)
        .antialiasing(true)
        .run()
}

struct Application {
    time: chrono::DateTime<Local>,
    widgets: Vec<Widget>,
    fullscreen: bool,
    main_window: Option<window::Id>,
    theme: Option<Theme>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(chrono::DateTime<Local>),
    OpenMainWindow,
    WindowOpened(Id),
    ChangeTheme(Theme),
    ToggleFullscreen,
}

impl Application {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::done(Message::OpenMainWindow))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick(local_time) => {
                if local_time != self.time {
                    self.time = local_time;
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
            Message::ChangeTheme(theme) => {
                self.theme = Some(theme);
                Task::none()
            }
            Message::WindowOpened(id) => {
                self.main_window = Some(id);
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
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(milliseconds(16)).map(|_| Message::Tick(chrono::Local::now()))
    }

    fn view(&self, _id: Id) -> Element<'_, Message> {
        match self.main_window {
            Some(_id) => responsive(move |size| {
                scrollable(row![
                    container(responsive(move |size| {
                        container(row![
                            center(self.widgets[0].view(self.time, size))
                                .align_x(Alignment::Center)
                                .align_y(Alignment::Center)
                                .width(Length::Fill)
                                .height(Length::Fill),
                            column![
                                container(button("fullscreen").on_press(Message::ToggleFullscreen))
                                    .width(Length::Fill)
                                    .align_x(Alignment::End),
                                center(self.widgets[1].view(self.time, size))
                                    .width(Length::Fill)
                                    .height(Length::Fill),
                            ]
                        ])
                        .style(|_| container::Style {
                            background: Some(Color::BLACK.into()),
                            ..Default::default()
                        })
                        .into()
                    }))
                    .width(size.width)
                    .height(size.height),
                    container(responsive(move |size| {
                        container(text("second page").size(size.width * 0.1))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .into()
                    }))
                    .width(size.width)
                    .height(size.height),
                ])
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::hidden(),
                ))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            })
            .into(),
            None => container(text("window is closed")).into(),
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Application {
            time: chrono::Local::now(),
            widgets: vec![
                Widget::Clock(ClockWidget::default()),
                Widget::Calendar(CalendarWidget),
            ],
            fullscreen: false,
            main_window: None,
            theme: Some(Theme::Moonfly),
        }
    }
}

enum Widget {
    Calendar(CalendarWidget),
    Clock(ClockWidget),
}

impl Widget {
    pub fn view(&self, time: chrono::DateTime<Local>, size: Size) -> Element<'_, Message> {
        match self {
            Widget::Clock(w) => w.view(time, size),
            Widget::Calendar(w) => w.view(time, size),
        }
    }
}

struct CalendarWidget;

impl CalendarWidget {
    fn view(&self, time: chrono::DateTime<Local>, size: Size) -> Element<'_, Message> {
        let columns = 7;
        let spacing = 1.0;
        let cell_width = (size.width - (columns as f32 - 1.0) * spacing) / columns as f32;
        let cell_height = cell_width * 0.6;
        let font_size = (size.width / 2.0).min(size.height) / 20.0;

        let mut grid: Grid<'_, Message, Theme, Renderer> =
            Grid::new().columns(columns).spacing(spacing);

        let weekdays = ["mo", "tu", "we", "th", "fr", "sa", "su"];

        let first_day_of_month = weekday_to_number(
            &NaiveDate::from_ymd_opt(time.year(), time.month(), 1)
                .unwrap()
                .weekday(),
        );

        let last_day_of_month = NaiveDate::from_ymd_opt(time.year(), time.month() + 1, 1)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(time.year() + 1, 1, 1).unwrap())
            .pred_opt()
            .unwrap()
            .day() as usize;

        for (ind, i) in weekdays.iter().enumerate() {
            grid = grid.push(
                container(text(*i).size(font_size).color(if ind == 5 || ind == 6 {
                    color!(87, 87, 87)
                } else {
                    Color::WHITE
                }))
                .width(cell_width)
                .height(cell_height)
                .center_x(cell_width)
                .center_y(cell_height),
            )
        }

        for _ in 0..first_day_of_month - 1 {
            grid = grid.push(container(""));
        }

        for i in 1..=last_day_of_month {
            if i == time.day() as usize {
                grid = grid.push(
                    container(text(i.to_string()).size(font_size).color(Color::WHITE))
                        .width(cell_width)
                        .height(cell_height)
                        .center_x(cell_width)
                        .center_y(cell_height)
                        .style(move |_| container::Style {
                            background: Some(color!(255, 0, 0).into()),
                            border: iced::Border {
                                radius: (cell_height * 0.9).into(),
                                width: 0.0,
                                color: iced::Color::TRANSPARENT,
                            },
                            ..Default::default()
                        }),
                );
            } else if (i + first_day_of_month - 1 - (i + first_day_of_month - 1) / 7) % 6 == 0 {
                grid = grid.push(
                    container(
                        text(i.to_string())
                            .size(font_size)
                            .color(color!(87, 87, 87)),
                    )
                    .height(cell_height)
                    .center_x(cell_width)
                    .center_y(cell_height),
                );
            } else {
                grid = grid.push(
                    container(text(i.to_string()).size(font_size).color(Color::WHITE))
                        .height(cell_height)
                        .center_x(cell_width)
                        .center_y(cell_height),
                );
            }
        }

        column![
            text(format!(" {}", time.format("%B")))
                .size(font_size * 2.0)
                .color(color!(255, 0, 0)),
            container(grid).width(size.width / 2.2)
        ]
        .into()
    }
}

struct ClockWidget {
    style: ClockStyle,
}

impl Default for ClockWidget {
    fn default() -> Self {
        Self {
            style: ClockStyle::Minimal(MinimalClock::default()),
        }
    }
}

impl ClockWidget {
    fn view(&self, _time: chrono::DateTime<Local>, _size: Size) -> Element<'_, Message> {
        self.style.view()
    }
}

enum ClockStyle {
    Digital(DigitalClock),
    Minimal(MinimalClock),
}

impl ClockStyle {
    fn view(&self) -> Element<'_, Message> {
        match self {
            ClockStyle::Digital(clock) => clock.view(),
            ClockStyle::Minimal(clock) => clock.view(),
        }
    }
}

#[derive(Default)]
struct DigitalClock {
    cache: Cache,
}

impl DigitalClock {
    fn view(&self) -> Element<'_, Message> {
        self.cache.clear();
        canvas(self as &Self)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for DigitalClock {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let clock = self.cache.draw(renderer, bounds.size(), |frame| {
            let palette = theme.extended_palette();

            let center = frame.center();
            let width = frame.width() / 2.0;

            let now = chrono::Local::now();
            let font_size = width * 0.6;

            // часы
            frame.fill_text(canvas::Text {
                content: format!("{:02}", now.hour()),
                position: Point {
                    x: center.x - font_size * 0.8,
                    y: center.y,
                },
                size: font_size.into(),
                color: palette.secondary.base.text,
                font: Font {
                    family: Family::Name("SF Pro Rounded"),
                    weight: Weight::Black,
                    ..Font::DEFAULT
                },
                align_x: text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            // двоеточие мигающее
            let colon = if now.second() % 2 == 0 { ":" } else { " " };
            frame.fill_text(canvas::Text {
                content: colon.to_string(),
                position: center,
                size: font_size.into(),
                color: color!(255, 0, 0),
                font: Font {
                    family: Family::Name("SF Pro Rounded"),
                    weight: Weight::Black,
                    ..Font::DEFAULT
                },
                align_x: text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });

            // минуты
            frame.fill_text(canvas::Text {
                content: format!("{:02}", now.minute()),
                position: Point {
                    x: center.x + font_size * 0.8,
                    y: center.y,
                },
                size: font_size.into(),
                color: palette.secondary.base.text,
                font: Font {
                    family: Family::Name("SF Pro Rounded"),
                    weight: Weight::Black,
                    ..Font::DEFAULT
                },
                align_x: text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
                ..Default::default()
            });
        });

        vec![clock]
    }
}

#[derive(Default)]
struct MinimalClock {
    hands: Hands,
    clock_frame: ClockFrame,
}

impl MinimalClock {
    fn view(&self) -> Element<'_, Message> {
        stack![self.clock_frame.view(), self.hands.view()].into()
    }
}

#[derive(Default)]
struct Hands {
    cache: Cache,
}

impl Hands {
    fn view(&self) -> Element<'_, Message> {
        self.cache.clear();

        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for Hands {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.extended_palette();

        let dynamic_layer = self.cache.draw(renderer, bounds.size(), |frame| {
            let now = chrono::Local::now();

            let minutes_portion = Radians::from(hand_rotation(now.minute(), 60)) / 12.0;

            let center = frame.center();

            let radius = frame.width().min(frame.height()) / 2.3;

            let hour_hand = Path::line(Point::ORIGIN, Point::new(0.0, -0.5 * radius));

            let minute_hand = Path::line(Point::ORIGIN, Point::new(0.0, -0.9 * radius));

            let second_hand = Path::line(Point::ORIGIN, Point::new(0.0, radius));

            let width = radius / 100.0;

            let thin_stroke = || -> Stroke {
                Stroke {
                    width: width,
                    style: stroke::Style::Solid(color!(240, 157, 10)),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                }
            };

            let wide_stroke = || -> Stroke {
                Stroke {
                    width: width * 5.0,
                    style: stroke::Style::Solid(palette.secondary.strong.text),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                }
            };

            frame.translate(Vector::new(center.x, center.y));

            let hour_hand_angle = Radians::from(hand_rotation(now.hour(), 12)) + minutes_portion;

            // часовая стрелка
            frame.with_save(|frame| {
                frame.rotate(hour_hand_angle);
                frame.stroke(&hour_hand, wide_stroke());
            });

            // минутная стрелка
            frame.with_save(|frame| {
                let minute_angle = hand_rotation(now.minute(), 60);

                frame.with_save(|f| {
                    f.rotate(minute_angle);
                    f.translate(Vector::new(-2.0, 0.0));
                    f.stroke(
                        &minute_hand,
                        Stroke {
                            width: width * 6.5,
                            style: stroke::Style::Solid(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.6,
                            }),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );
                });

                frame.rotate(minute_angle);
                frame.stroke(&minute_hand, wide_stroke());
            });

            // секундная стрелка
            frame.with_save(|frame| {
                let seconds = now.second() as f32 + now.nanosecond() as f32 / 1_000_000_000.0;
                let rotation = hand_rotation_sec(seconds, 60.0);

                frame.with_save(|f| {
                    f.rotate(rotation);
                    f.translate(Vector::new(2.0, 2.0));
                    f.stroke(
                        &second_hand,
                        Stroke {
                            width: width * 1.2,
                            style: stroke::Style::Solid(Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.3,
                            }),
                            line_cap: LineCap::Round,
                            ..Stroke::default()
                        },
                    );
                });

                frame.rotate(rotation);
                frame.stroke(&second_hand, thin_stroke());
            });
        });

        vec![dynamic_layer]
    }
}

#[derive(Default)]
struct ClockFrame {
    cache: Cache,
}

impl ClockFrame {
    fn view(&self) -> Element<'_, Message> {
        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for ClockFrame {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.extended_palette();

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
                    position: Point::new(x * 0.8, y * 0.8),
                    color: palette.secondary.strong.text,
                    align_x: text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    font: Font {
                        family: Family::Name("SF Pro Rounded"),
                        weight: Weight::Black,
                        ..Font::DEFAULT
                    },
                    ..canvas::Text::default()
                });
            }

            for tick in 0..60 {
                let angle = hand_rotation(tick, 60);
                let width = if tick % 5 == 0 {
                    radius * 0.016
                } else {
                    radius * 0.0095
                };

                frame.with_save(|frame| {
                    frame.rotate(angle);
                    frame.fill(
                        &Path::rectangle(Point::new(0.0, radius), Size::new(width, width * 6.0)),
                        palette.secondary.strong.text,
                    );
                });
            }
        });

        vec![static_layer]
    }
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

fn hand_rotation_sec(value: f32, max: f32) -> iced::Radians {
    iced::Radians(value / max * std::f32::consts::TAU)
}
