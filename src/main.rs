use chrono::prelude::*;
use iced::advanced::Renderer as AdvancedRenderer;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self};
use iced::advanced::{Clipboard, Shell};
use iced::border::Radius;
use iced::font::{Family, Weight};
use iced::mouse;
use iced::time::{self, milliseconds};
use iced::widget::canvas::{Cache, LineCap, Path, Stroke, stroke};
use iced::widget::{Grid, button, canvas, center, column, container, responsive, row, stack, text};
use iced::window::{self, Id};
use iced::{
    Alignment, Color, Degrees, Element, Event, Font, Length, Point, Radians, Rectangle, Renderer,
    Settings, Size, Subscription, Task, Theme, Vector, color,
};
use std::time::{Duration, Instant};

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
    widgets_1: Vec<AppWidget>,
    widgets_2: Vec<AppWidget>,
    fullscreen: bool,
    main_window: Option<window::Id>,
    current_page: usize,
    page_width: f32,
    drag: DragState,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(chrono::DateTime<Local>),
    OpenMainWindow,
    WindowOpened(Id),
    ToggleFullscreen,
    DragDelta(f32),
    SnapTick(Instant),
    AnimTick(Instant),
    UpdatePageWidth(f32),
}

impl Application {
    fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::done(Message::OpenMainWindow))
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
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let clock = time::every(milliseconds(16)).map(|_| Message::Tick(chrono::Local::now()));
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
        Subscription::batch([clock, snap_idle, anim])
    }

    fn view(&self, _id: Id) -> Element<'_, Message> {
        match self.main_window {
            Some(_id) => responsive(move |size| {
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
                        let t = (elapsed / (SNAP_DURATION_MS as f32 / 1000.0)).min(1.0);
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
            })
            .into(),
            None => container(text("window is closed")).into(),
        }
    }

    fn page0(&self, size: Size) -> Element<'_, Message> {
        container(responsive(move |size| {
            container(row![
                center(self.widgets_1[0].view(self.time, size))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .height(Length::Fill),
                column![
                    container(button("fullscreen").on_press(Message::ToggleFullscreen))
                        .width(Length::Fill)
                        .align_x(Alignment::End),
                    center(self.widgets_1[1].view(self.time, size))
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
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn page1(&self, _size: Size) -> Element<'_, Message> {
        container(responsive(move |size| {
            self.widgets_2[0].view(self.time, size)
        }))
        .style(|_| container::Style {
            background: Some(Color::BLACK.into()),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

impl Default for Application {
    fn default() -> Self {
        Application {
            time: chrono::Local::now(),
            widgets_1: vec![
                AppWidget::Clock(ClockWidget::default()),
                AppWidget::Calendar(CalendarWidget),
            ],
            widgets_2: vec![AppWidget::Clock(ClockWidget::new(
                ClockStyle::AnalogueFull(AnalogueClockFull::default()),
            ))],
            fullscreen: false,
            main_window: None,
            current_page: 0,
            page_width: 800.0,
            drag: DragState::Idle,
        }
    }
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
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();

        shell.publish(Message::UpdatePageWidth(bounds.width));

        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
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
        self.children
            .iter()
            .zip(layout.children())
            .enumerate()
            .map(|(i, (child, child_layout))| {
                child.as_widget().mouse_interaction(
                    &tree.children[i],
                    child_layout,
                    cursor,
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
}

impl AppWidget {
    pub fn view(&self, time: chrono::DateTime<Local>, size: Size) -> Element<'_, Message> {
        match self {
            AppWidget::Clock(w) => w.view(time, size),
            AppWidget::Calendar(w) => w.view(time, size),
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
            style: ClockStyle::AnalogueHalf(AnalogueClockHalf::default()),
        }
    }
}

impl ClockWidget {
    fn new(style: ClockStyle) -> Self {
        Self { style }
    }

    fn view(&self, _time: chrono::DateTime<Local>, _size: Size) -> Element<'_, Message> {
        self.style.view()
    }
}

enum ClockStyle {
    DigitalHalf(DigitalClockHalf),
    AnalogueHalf(AnalogueClockHalf),
    MinimalHalf(MinimalClockHalf),
    AnalogueFull(AnalogueClockFull),
}

impl ClockStyle {
    fn view(&self) -> Element<'_, Message> {
        match self {
            ClockStyle::DigitalHalf(clock) => clock.view(),
            ClockStyle::AnalogueHalf(clock) => clock.view(),
            ClockStyle::MinimalHalf(clock) => clock.view(),
            ClockStyle::AnalogueFull(clock) => clock.view(),
        }
    }
}

#[derive(Default)]
struct DigitalClockHalf {
    cache: Cache,
}

impl DigitalClockHalf {
    fn view(&self) -> Element<'_, Message> {
        self.cache.clear();
        canvas(self as &Self)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for DigitalClockHalf {
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
struct AnalogueClockHalf {
    hands: Hands,
    clock_frame: ClockFrameAnalogueHalf,
}

impl AnalogueClockHalf {
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
                let minute_angle = hand_rotation(now.minute() * 15 + now.second() / 4, 900);

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
                let rotation =
                    hand_rotation_sec(seconds, 60.0).0 - std::f32::consts::FRAC_PI_2 * 2.0;

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
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.extended_palette();

        let static_layer = self.cache.draw(renderer, bounds.size(), |frame| {
            let center = frame.center();

            frame.translate(Vector::new(center.x, center.y));

            let radius = frame.width().min(frame.height()) / 2.4;

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

            let mut color;

            for tick in 0..60 {
                let angle = hand_rotation(tick, 60);
                let width = if tick % 5 == 0 {
                    color = palette.secondary.strong.text;
                    radius * 0.016
                } else {
                    color = palette.secondary.base.color;
                    radius * 0.0095
                };

                frame.with_save(|frame| {
                    frame.rotate(angle);
                    frame.fill(
                        &Path::rounded_rectangle(
                            Point::new(0.0, radius),
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
    fn view(&self) -> Element<'_, Message> {
        stack![self.clock_frame.view(), self.hands.view()].into()
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
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let palette = theme.extended_palette();

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
                        palette.secondary.strong.text,
                    );
                });
            }
        });

        vec![static_layer]
    }
}

#[derive(Default)]
struct AnalogueClockFull {
    hands: Hands,
    clock_frame: ClockFrameAnalogueFull,
}

impl AnalogueClockFull {
    fn view(&self) -> Element<'_, Message> {
        stack![self.clock_frame.view(), self.hands.view()].into()
    }
}

#[derive(Default)]
struct ClockFrameAnalogueFull {
    cache: Cache,
}

impl ClockFrameAnalogueFull {
    fn view(&self) -> Element<'_, Message> {
        canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<Message> canvas::Program<Message> for ClockFrameAnalogueFull {
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
            let padding = 70.0;
            let inner_padding_hour = 250.0;
            let inner_padding_min = 120.0;

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
            let height = frame.height() - padding * 2.0;

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
                            .with_color(palette.secondary.base.color)
                            .with_width(4.0)
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
                            point.x + (inner_padding_hour - point.y) * (dx / dy),
                            inner_padding_hour,
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.secondary.strong.text)
                            .with_width(10.0)
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
                            .with_color(palette.secondary.base.color)
                            .with_width(4.0)
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
                            point.x + ((frame.height() - inner_padding_hour) - point.y) * (dx / dy),
                            frame.height() - inner_padding_hour,
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(palette.secondary.strong.text)
                            .with_width(10.0)
                            .with_line_cap(LineCap::Round),
                    );
                }

                //left side
                for i in 1..10 {
                    let point = Point::new(top_left.x, top_left.y + height * 0.1 * i as f32);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if i == 5 {
                        Point::new(point.x + inner_padding_min, point.y)
                    } else {
                        Point::new(
                            inner_padding_min,
                            point.y + (inner_padding_min - point.x) * (dy / dx),
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(if i == 5 {
                                palette.secondary.strong.text
                            } else {
                                palette.secondary.base.color
                            })
                            .with_width(if i == 5 { 10.0 } else { 4.0 })
                            .with_line_cap(LineCap::Round),
                    );
                }

                //right side
                for i in 1..10 {
                    let point =
                        Point::new(top_left.x + width, top_left.y + height * 0.1 * i as f32);

                    let dx = center.x - point.x;
                    let dy = center.y - point.y;

                    let end_point = if i == 5 {
                        Point::new(point.x - inner_padding_min, point.y)
                    } else {
                        Point::new(
                            frame.width() - inner_padding_min,
                            point.y + ((frame.width() - inner_padding_min) - point.x) * (dy / dx),
                        )
                    };

                    let line = Path::line(point, end_point);

                    frame.stroke(
                        &line,
                        Stroke::default()
                            .with_color(if i == 5 {
                                palette.secondary.strong.text
                            } else {
                                palette.secondary.base.color
                            })
                            .with_width(if i == 5 { 10.0 } else { 4.0 })
                            .with_line_cap(LineCap::Round),
                    );
                }
            })
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
