use chrono::prelude::*;
use iced::font::{Family, Weight};
use iced::time::{self, seconds};
use iced::widget::canvas::Cache;
use iced::widget::{Grid, button, center, column, container, responsive, row, text};
use iced::window::{self, Id};
use iced::{
    Alignment, Color, Element, Font, Length, Renderer, Settings, Size, Subscription, Task, Theme,
    color,
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
    ChangeTheme(Theme),
    OpenMainWindow,
    WindowOpened(Id),
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
            Message::ChangeTheme(theme) => {
                self.theme = Some(theme);
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
                    ..Default::default()
                });

                self.main_window = Some(id);

                task.map(move |_| Message::WindowOpened(id))
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
        time::every(seconds(1)).map(|_| Message::Tick(chrono::Local::now()))
    }

    fn view(&self, _id: Id) -> Element<'_, Message> {
        match self.main_window {
            Some(_id) => responsive(move |size| {
                row![
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
                ]
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
                Widget::Clock(ClockWidget {
                    static_cache: Cache::default(),
                    dynamic_cache: Cache::default(),
                }),
                Widget::Calendar(CalendarWidget {
                    cache: Cache::default(),
                }),
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

#[derive(Debug)]
struct CalendarWidget {
    cache: Cache,
}

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

#[derive(Debug)]
struct ClockWidget {
    static_cache: Cache,  //for storing static elements
    dynamic_cache: Cache, //for storing dynamic elements
}

impl ClockWidget {
    fn view(&self, time: chrono::DateTime<Local>, size: Size) -> Element<'_, Message> {
        center(text(format!("{}", time.date_naive())))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
