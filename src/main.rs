use chrono::{self, Local};
use iced::alignment::Vertical::Center;
use iced::font::{Family, Weight};
use iced::time::{self, seconds};
use iced::widget::canvas::Cache;
use iced::widget::{Grid, button, center, column, container, responsive, row, text};
use iced::window::{self, Id};
use iced::{Alignment, Element, Font, Renderer, Settings, Subscription, Task, Theme};

pub fn main() -> iced::Result {
    iced::application(Application::default, Application::update, Application::view)
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
        // .window(window::Settings {
        //     fullscreen: true,
        //     ..Default::default()
        // })
        .run()
}

struct Application {
    time: chrono::DateTime<Local>,
    static_cache: Cache, //cache for storing static elements
    dynamic_cache: Cache,
    fullscreen: bool,
    // window_id: Option<window::Id>,
    theme: Option<Theme>,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(chrono::DateTime<Local>),
    ChangeTheme(Theme),
    // WindowEvent(iced::Event),
    ToggleFullscreen,
}

impl Application {
    fn new() -> Self {
        Application {
            time: chrono::Local::now(),
            static_cache: Cache::default(),
            dynamic_cache: Cache::default(),
            fullscreen: false,
            // window_id: None,
            theme: Some(Theme::Moonfly),
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::Tick(local_time) => {
                if local_time != self.time {
                    self.time = local_time;
                }
                // Task::none()
            }
            Message::ChangeTheme(theme) => {
                self.theme = Some(theme);
                // Task::none()
            }

            // Message::WindowEvent(event) => {
            //     if let iced::Event::Window(id, window_event) = event {
            //         // Сохраняем ID при первом событии
            //         if self.window_id.is_none() {
            //             println!("Получен ID окна из события: {:?}", id);
            //             self.window_id = Some(id);
            //         }
            //     }
            // }
            Message::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;

                // if let Some(id) = self.window_id {
                if self.fullscreen {
                    window::set_mode::<Message>(Id::unique(), window::Mode::Fullscreen);
                } else {
                    window::set_mode::<Message>(Id::unique(), window::Mode::Windowed);
                }
                // }

                // window::set_mode(self.window_id, window::Mode::Fullscreen);
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // iced::event::listen().map(Message::WindowEvent);

        time::every(seconds(1)).map(|_| Message::Tick(chrono::Local::now()))
    }

    fn view(&self) -> Element<'_, Message> {
        responsive(|size| {
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
        .into()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}
