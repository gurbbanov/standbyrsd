use chrono::{self, Local};
use iced::alignment::Vertical::Center;
use iced::font::{Family, Weight};
use iced::time::{self, seconds};
use iced::widget::canvas::Cache;
use iced::widget::{Grid, button, center, column, container, responsive, row, text};
use iced::window::{self, Id};
use iced::{Alignment, Element, Font, Renderer, Settings, Size, Subscription, Task, Theme};

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
    static_cache: Cache, //cache for storing static elements
    dynamic_cache: Cache,
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
                    size: Size {
                        width: 800.0,
                        height: 600.0,
                    },
                    position: window::Position::Centered,
                    ..Default::default()
                });

                self.main_window = Some(id);

                // Task::none()
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
            Some(_id) => responsive(|size| {
                let columns = 7;
                let spacing = 1.0;
                let cell_width = (size.width - (columns as f32 - 1.0) * spacing) / columns as f32;
                let cell_height = cell_width * 0.4;
                let font_size = (size.width / 2.0).min(size.height) / 22.0;

                let mut grid: Grid<'_, Message, Theme, Renderer> =
                    Grid::new().columns(columns).spacing(spacing);

                for i in 1..31 {
                    grid = grid.push(
                        container(text(i.to_string()).size(font_size))
                            .height(cell_height)
                            .center_x(cell_width)
                            .center_y(cell_height),
                    );
                }

                row![
                    center(column![
                        text(format!("curr time: {}", self.time)),
                        button("fullscreen").on_press(Message::ToggleFullscreen)
                    ])
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
                    center(container(grid).width(size.width / 2.5))
                ]
                .align_y(Center)
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
            static_cache: Cache::default(),
            dynamic_cache: Cache::default(),
            fullscreen: false,
            main_window: None,
            theme: Some(Theme::Moonfly),
        }
    }
}
