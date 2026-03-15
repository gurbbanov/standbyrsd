use chrono::prelude::*;
use iced::advanced::Renderer as AdvancedRenderer;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self};
use iced::advanced::{Clipboard, Shell};
use iced::border::Radius;
use iced::font::{Family, Weight};
use iced::time::{self, milliseconds, seconds};
use iced::widget::canvas::{Cache, LineCap, Path, Stroke, stroke};
use iced::widget::{button, canvas, center, column, container, responsive, row, stack, text};
use iced::window::{self, Id};
use iced::{
    Alignment, Color, Degrees, Element, Event, Font, Length, Point, Radians, Rectangle, Renderer,
    Settings, Size, Subscription, Task, Theme, Vector, color,
};
use iced::{Pixels, mouse};
use reqwest;
use serde::Deserialize;
use std::cell::Cell;
use std::time::{Duration, Instant};

const SF_PRO_EXPANDED_BOLD: Font = Font {
    family: iced::font::Family::Name("SF Pro"),
    weight: iced::font::Weight::Bold,
    stretch: iced::font::Stretch::Expanded,
    style: iced::font::Style::Normal,
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

pub fn main() -> iced::Result {
    iced::daemon(Application::new, Application::update, Application::view)
        .subscription(Application::subscription)
        .settings(Settings {
            fonts: vec![
                include_bytes!("../fonts/SF-Pro-Rounded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Expanded.ttf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Black.otf").into(),
                include_bytes!("../fonts/SF-Pro-Display-Bold.otf").into(),
            ],
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
    weather: WeatherStatus,
    page0_left: Vec<AppWidget>,
    page0_right: Vec<AppWidget>,
    page1_widgets: Vec<AppWidget>,
    fullscreen: bool,
    main_window: Option<window::Id>,
    current_page: usize,
    page_width: f32,
    drag: DragState,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(chrono::DateTime<Local>),
    FetchWeather,
    WeatherFetched(WeatherStatus),
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
        (
            Self::default(),
            Task::batch([
                Task::done(Message::OpenMainWindow),
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
        let weather = time::every(seconds(600)).map(|_| Message::FetchWeather);
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
        Subscription::batch([clock, weather, snap_idle, anim])
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
        let sh = size.height;
        let sw = size.width / 2.0;

        let left_items: Vec<Element<'_, Message>> = self
            .page0_left
            .iter()
            .map(|w| {
                container(w.view(&self.time, &self.weather, size))
                    .width(Length::Fixed(sw))
                    .height(Length::Fixed(sh))
                    .style(|_| container::Style {
                        background: Some(Color::BLACK.into()),
                        ..Default::default()
                    })
                    .into()
            })
            .collect();

        let right_items: Vec<Element<'_, Message>> = self
            .page0_right
            .iter()
            .map(|w| {
                container(w.view(&self.time, &self.weather, size))
                    .width(Length::Fixed(sw))
                    .height(Length::Fixed(sh))
                    .style(|_| container::Style {
                        background: Some(Color::BLACK.into()),
                        ..Default::default()
                    })
                    .into()
            })
            .collect();

        let left = vertical_carousel(left_items, sw, sh);
        let right = vertical_carousel(right_items, sw, sh);

        container(row![
            left,
            column![
                container(button("fullscreen").on_press(Message::ToggleFullscreen))
                    .width(Length::Fill)
                    .align_x(Alignment::End),
                right,
            ]
            .width(Length::Fixed(sw))
            .height(Length::Fixed(sh)),
        ])
        .style(|_| container::Style {
            background: Some(Color::BLACK.into()),
            ..Default::default()
        })
        .width(Length::Fixed(size.width))
        .height(Length::Fixed(size.height))
        .into()
    }

    fn page1(&self, size: Size) -> Element<'_, Message> {
        let items: Vec<Element<'_, Message>> = self
            .page1_widgets
            .iter()
            .map(|w| {
                container(w.view(&self.time, &self.weather, size))
                    .width(Length::Fixed(size.width))
                    .height(Length::Fixed(size.height))
                    .style(|_| container::Style {
                        background: Some(Color::BLACK.into()),
                        ..Default::default()
                    })
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
                AppWidget::Clock(ClockWidget::new(ClockStyle::DigitalHalf(
                    DigitalClockHalf::default(),
                ))),
            ],
            page0_right: vec![
                AppWidget::Calendar(CalendarWidget::default()),
                AppWidget::Weather(WeatherWidget::default()),
                AppWidget::Weather(WeatherWidget::new(WeatherStyle::DetailedHalf(
                    DetailedForecastHalf::default(),
                ))),
                AppWidget::Weather(WeatherWidget::new(WeatherStyle::DailyHalf(
                    DailyForecastHalf::default(),
                ))),
            ],
            page1_widgets: vec![
                AppWidget::Clock(ClockWidget::new(ClockStyle::AnalogueFull(
                    AnalogueClockFull::default(),
                ))),
                AppWidget::Clock(ClockWidget::new(ClockStyle::AnalogueFull(
                    AnalogueClockFull::default(),
                ))),
            ],
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
        event: &Event,
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

        {
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

            if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
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
        }

        for (i, (child, child_layout)) in self.items.iter_mut().zip(layout.children()).enumerate() {
            child.as_widget_mut().update(
                &mut tree.children[i],
                event,
                child_layout,
                cursor,
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
        self.items
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
    Weather(WeatherWidget),
}

impl AppWidget {
    pub fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        match self {
            AppWidget::Clock(w) => w.view(time),
            AppWidget::Calendar(w) => w.view(time),
            AppWidget::Weather(w) => w.view(time, weather, size),
        }
    }
}

#[derive(Default)]
struct CalendarWidget {
    last_day: Cell<u32>,
    cache: Cache,
}

impl CalendarWidget {
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

impl<'a> canvas::Program<Message> for (&'a CalendarWidget, &'a DateTime<Local>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, now) = self;

        let layer = widget.cache.draw(renderer, bounds.size(), |frame| {
            let w = frame.width();
            let h = frame.height() * 0.9;

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
            let month_font_size = cell_w * 0.7;

            let grid_w = cell_w * columns as f32;
            let total_h = month_font_size + cell_h * (1.0 + num_rows as f32);
            let offset_x = (w - grid_w) * 0.5;
            let offset_y = (h - total_h) * 0.5;

            frame.fill_text(canvas::Text {
                content: format!("{}", now.format("%B")),
                position: Point::new(offset_x * 1.3, offset_y + month_font_size * 0.5),
                size: month_font_size.into(),
                color: color!(255, 0, 0),
                font: SF_PRO_ROUNDED_BLACK,
                align_x: text::Alignment::Left,
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });

            let weekdays = ["mo", "tu", "we", "th", "fr", "sa", "su"];

            for (col, label) in weekdays.iter().enumerate() {
                let x = offset_x + col as f32 * cell_w + cell_w * 0.5;
                let y = offset_y + month_font_size + cell_h * 0.5;
                let is_weekend = col >= 5;
                frame.fill_text(canvas::Text {
                    content: label.to_string(),
                    position: Point::new(x, y),
                    size: font_size.into(),
                    color: if is_weekend {
                        color!(87, 87, 87)
                    } else {
                        Color::WHITE
                    },
                    font: SF_PRO_ROUNDED_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
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
                        Color::WHITE
                    } else if is_weekend {
                        color!(87, 87, 87)
                    } else {
                        Color::WHITE
                    },
                    font: SF_PRO_ROUNDED_BLACK,
                    align_x: text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });

                slot += 1;
            }
        });

        vec![layer]
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

    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        self.style.view(time)
    }
}

enum ClockStyle {
    DigitalHalf(DigitalClockHalf),
    AnalogueHalf(AnalogueClockHalf),
    MinimalHalf(MinimalClockHalf),
    AnalogueFull(AnalogueClockFull),
}

impl ClockStyle {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        match self {
            ClockStyle::DigitalHalf(clock) => clock.view(time),
            ClockStyle::AnalogueHalf(clock) => clock.view(time),
            ClockStyle::MinimalHalf(clock) => clock.view(time),
            ClockStyle::AnalogueFull(clock) => clock.view(time),
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
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, now) = self;

        let clock = widget.cache.draw(renderer, bounds.size(), |frame| {
            let palette = theme.extended_palette();

            let center = frame.center();
            let width = frame.width() / 2.0;

            let font_size = width * 0.6;

            // часы
            frame.fill_text(canvas::Text {
                content: format!("{:02}", now.hour()),
                position: Point {
                    x: center.x - font_size * 0.2,
                    y: center.y,
                },
                size: font_size.into(),
                color: palette.secondary.base.text,
                font: SF_PRO_ROUNDED_BLACK,
                align_x: text::Alignment::Right,
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
                font: SF_PRO_ROUNDED_BLACK,
                align_x: text::Alignment::Center,
                align_y: iced::alignment::Vertical::Center,
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
                color: palette.secondary.base.text,
                font: SF_PRO_ROUNDED_BLACK,
                align_x: text::Alignment::Left,
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
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        stack![self.clock_frame.view(), self.hands.view(time)].into()
    }
}

#[derive(Default)]
struct Hands {
    cache: Cache,
}

impl Hands {
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        self.cache.clear();

        canvas((self, time))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl<'a> canvas::Program<Message> for (&'a Hands, &'a DateTime<Local>) {
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
        let (widget, now) = self;

        let dynamic_layer = widget.cache.draw(renderer, bounds.size(), |frame| {
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
                    font: SF_PRO_ROUNDED_BLACK,
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
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        stack![self.clock_frame.view(), self.hands.view(time)].into()
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
    fn view<'a>(&'a self, time: &'a DateTime<Local>) -> Element<'a, Message> {
        stack![self.clock_frame.view(time), self.hands.view(time)].into()
    }
}

#[derive(Default)]
struct ClockFrameAnalogueFull {
    last_day: Cell<u32>,
    cache: Cache,
}

impl ClockFrameAnalogueFull {
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

impl<'a> canvas::Program<Message> for (&'a ClockFrameAnalogueFull, &'a DateTime<Local>) {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let (widget, time) = self;
        let palette = theme.extended_palette();

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
                            .with_color(palette.secondary.base.color)
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
                            .with_color(color!(169, 169, 169))
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
                            .with_color(color!(89, 89, 89))
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
                            .with_color(color!(169, 169, 169))
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
                                color!(169, 169, 169)
                            } else {
                                color!(89, 89, 89)
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
                                color!(169, 169, 169)
                            } else {
                                color!(89, 89, 89)
                            })
                            .with_width(if i == 5 { 10.0 * scale } else { 4.0 * scale })
                            .with_line_cap(LineCap::Round),
                    );
                }

                frame.fill_text(canvas::Text {
                    content: time.weekday().to_string().to_uppercase(),
                    size: iced::Pixels(50.0 * scale),
                    position: Point::new(frame.width() * 2.0 / 3.0, frame.center().y),
                    color: color!(255, 0, 0),
                    align_x: text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
                    font: SF_PRO_EXPANDED_BOLD,
                    ..canvas::Text::default()
                });

                frame.fill_text(canvas::Text {
                    content: time.day().to_string(),
                    size: iced::Pixels(50.0 * scale),
                    position: Point::new(
                        frame.width() * 2.0 / 3.0 + 110.0 * scale,
                        frame.center().y,
                    ),
                    color: Color::WHITE,
                    align_x: text::Alignment::Center,
                    align_y: iced::alignment::Vertical::Center,
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
                        size: iced::Pixels(125.0 * scale),
                        position: point,
                        color: palette.secondary.strong.text,
                        align_x: text::Alignment::Center,
                        align_y: iced::alignment::Vertical::Center,
                        font: SF_PRO_EXPANDED_BOLD,
                        ..canvas::Text::default()
                    });
                }
            })
        });

        vec![static_layer]
    }
}

#[derive(Clone, Debug, Deserialize, Default)]
struct Weather {
    city: Option<String>,
    current: Option<CurrentForecast>,
    daily: Option<DailyForecast>,
}

impl Weather {
    async fn fetch(&mut self) -> Result<(), reqwest::Error> {
        let ip = reqwest::get("https://api.ipify.org").await?.text().await?;

        let info = geolocation::find(&ip).unwrap();

        let response: Weather = reqwest::get(
            format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=precipitation_probability_max,apparent_temperature_max,apparent_temperature_min,weather_code,uv_index_max&current=temperature_2m,is_day,wind_speed_10m,precipitation,weather_code,apparent_temperature", info.latitude, info.longitude),
        ).await?.json::<Self>().await?;

        *self = Weather {
            city: Some(info.city.replace("\"", "")),
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
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        self.style.view(time, weather, size)
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
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        match self {
            Self::MinimalHalf(w) => w.view(weather, size),
            Self::DetailedHalf(w) => w.view(weather, size),
            Self::DailyHalf(w) => w.view(time, weather, size),
        }
    }
}

#[derive(Default)]
struct MinimalForecastHalf {
    cache: Cache,
}

impl MinimalForecastHalf {
    fn view<'a>(&'a self, weather: &'a WeatherStatus, size: Size) -> Element<'a, Message> {
        let w = size.width / 2.0;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 100.0 * scale;
        let icon_x = w * 0.05;
        let icon_y = h / 2.0 + 200.0 * scale - icon_size - 20.0 * scale;

        let icon: Element<Message> = match weather {
            WeatherStatus::Ok(w_data) => {
                let code = w_data.current.as_ref().unwrap().weather_code;
                if (code == 0 || code == 1) && w_data.current.as_ref().unwrap().is_day == 0 {
                    iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(100)))
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                } else {
                    iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(code)))
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                }
            }
            WeatherStatus::Error(e) => button("Retry").on_press(Message::FetchWeather).into(),
            _ => iced::widget::svg(iced::widget::svg::Handle::from_memory(include_bytes!(
                "../icons/clear.svg"
            )))
            .into(),
        };

        stack![
            canvas((self, weather))
                .width(Length::Fill)
                .height(Length::Fill),
            container(icon)
                .padding(iced::padding::top(icon_y).left(icon_x))
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
                            frame.center().y - 380.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.33),
                        position: Point::new(w * 0.05, frame.center().y),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}", wmo_code_description(current.weather_code)),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 300.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
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
                            frame.center().y + 390.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });
                })
            }),
            WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {}),
            WeatherStatus::Error(e) => widget.cache.draw(renderer, bounds.size(), |frame| {}),
        };
        vec![static_layer]
    }
}

#[derive(Default)]
struct DetailedForecastHalf {
    cache: Cache,
}

impl DetailedForecastHalf {
    fn view<'a>(&'a self, weather: &'a WeatherStatus, size: Size) -> Element<'a, Message> {
        let w = size.width / 2.0;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 80.0 * scale;
        let icon_x = w * 0.83;
        let icon_y = h / 2.0 - 380.0 * scale - icon_size - 20.0 * scale;

        let icon = match weather {
            WeatherStatus::Ok(w_data) => {
                let code = w_data.current.as_ref().unwrap().weather_code;
                if (code == 0 || code == 1) && w_data.current.as_ref().unwrap().is_day == 0 {
                    iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(100)))
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                } else {
                    iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(code)))
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into()
                }
            }
            _ => iced::widget::svg(iced::widget::svg::Handle::from_memory(include_bytes!(
                "../icons/clear.svg"
            ))),
        };

        stack![
            canvas((self, weather))
                .width(Length::Fill)
                .height(Length::Fill),
            container(icon)
                .padding(iced::padding::top(icon_y).left(icon_x))
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
        let palette = theme.extended_palette();

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
                            frame.center().y - 380.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.2),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 180.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↑{:.0}°", daily.apparent_temperature_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 300.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↓{:.0}°", daily.apparent_temperature_min[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 200.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Precip"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 100.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
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
                                format!("{}% in {}d", v, i)
                            }),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 100.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Wind"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 50.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{} m/s", current.wind_speed_10m),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 50.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("UVI"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 200.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{}", daily.uv_index_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 200.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("Feels Like"),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y + 350.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.apparent_temperature),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y + 350.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });
                });
            }),
            WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {}),
            WeatherStatus::Error(e) => widget.cache.draw(renderer, bounds.size(), |frame| {}),
        };
        vec![static_layer]
    }
}

#[derive(Default)]
struct DailyForecastHalf {
    cache: Cache,
}

impl DailyForecastHalf {
    fn view<'a>(
        &'a self,
        time: &'a DateTime<Local>,
        weather: &'a WeatherStatus,
        size: Size,
    ) -> Element<'a, Message> {
        let w = size.width / 2.0;
        let h = size.height;
        let scale = (w / 960.0).min(h / 1080.0);

        let icon_size = 80.0 * scale;
        let icon_x = w * 0.83;
        let icon_y = h / 2.0 - 380.0 * scale - icon_size - 20.0 * scale;

        let (icon, daily_icons): (Element<Message>, Vec<Element<Message>>) = match weather {
            WeatherStatus::Ok(w_data) => {
                let current = w_data.current.as_ref().unwrap();
                let code = if (current.weather_code == 0 || current.weather_code == 1)
                    && current.is_day == 0
                {
                    100u8
                } else {
                    current.weather_code as u8
                };

                let current_icon =
                    iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(code)))
                        .width(Length::Fixed(icon_size))
                        .height(Length::Fixed(icon_size))
                        .into();

                let daily = w_data.daily.as_ref();

                let icons = match daily {
                    Some(d) => (1..=4)
                        .filter_map(|i| d.weather_code.get(i).copied())
                        .map(|code| {
                            iced::widget::svg(iced::widget::svg::Handle::from_memory(wmo_code_svg(
                                code as u8,
                            )))
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
                iced::widget::svg(iced::widget::svg::Handle::from_memory(include_bytes!(
                    "../icons/clear.svg"
                )))
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
                .padding(iced::padding::top(icon_y).left(icon_x))
                .width(Length::Fill)
                .height(Length::Fill),
            container(daily_column)
                .padding(iced::padding::top(h / 2.0 - 180.0 * scale - 20.0 * scale).left(w * 0.3))
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
        let palette = theme.extended_palette();

        let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let today = weekday_to_number(&time.weekday());

        let mut curr_padding = -100.0;
        let mut counter = 0;

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
                            frame.center().y - 380.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("{:.0}°", current.temperature_2m),
                        size: Pixels(w.min(h) * 0.2),
                        position: Point::new(
                            w * 0.05,
                            frame.center().y - 180.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        font: SF_PRO_DISPLAY_BLACK,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↑{:.0}°", daily.apparent_temperature_max[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 300.0 * scale.min(h / 1080.0),
                        ),
                        color: Color::WHITE,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    frame.fill_text(canvas::Text {
                        content: format!("↓{:.0}°", daily.apparent_temperature_min[0]),
                        size: Pixels(w.min(h) * 0.08),
                        position: Point::new(
                            w * 0.95,
                            frame.center().y - 200.0 * scale.min(h / 1080.0),
                        ),
                        color: palette.secondary.base.color,
                        align_y: iced::alignment::Vertical::Bottom,
                        align_x: text::Alignment::Right,
                        font: SF_PRO_DISPLAY_BOLD,
                        ..canvas::Text::default()
                    });

                    for weekday in today + 1..7 {
                        frame.fill_text(canvas::Text {
                            content: format!("{}", weekdays[weekday]),
                            size: Pixels(w.min(h) * 0.08),
                            position: Point::new(
                                w * 0.05,
                                frame.center().y + curr_padding * scale.min(h / 1080.0),
                            ),
                            color: Color::WHITE,
                            align_y: iced::alignment::Vertical::Bottom,
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
                            color: palette.secondary.base.color,
                            align_y: iced::alignment::Vertical::Bottom,
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
                            color: Color::WHITE,
                            align_y: iced::alignment::Vertical::Bottom,
                            align_x: text::Alignment::Right,
                            font: SF_PRO_DISPLAY_BOLD,
                            ..canvas::Text::default()
                        });

                        curr_padding += 150.0;
                        counter += 1;
                        if counter == 4 {
                            break;
                        }
                    }

                    if counter != 4 {
                        for weekday in 0..(4 as i32 - counter as i32).abs() as usize {
                            frame.fill_text(canvas::Text {
                                content: format!("{}", weekdays[weekday]),
                                size: Pixels(w.min(h) * 0.08),
                                position: Point::new(
                                    w * 0.05,
                                    frame.center().y + curr_padding * scale.min(h / 1080.0),
                                ),
                                color: Color::WHITE,
                                align_y: iced::alignment::Vertical::Bottom,
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
                                color: palette.secondary.base.color,
                                align_y: iced::alignment::Vertical::Bottom,
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
                                color: Color::WHITE,
                                align_y: iced::alignment::Vertical::Bottom,
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
            WeatherStatus::Loading => widget.cache.draw(renderer, bounds.size(), |frame| {}),
            WeatherStatus::Error(e) => widget.cache.draw(renderer, bounds.size(), |frame| {}),
        };
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
        _ => include_bytes!("../icons/clear.svg"),
    }
}

fn arrow_svg(direction: &str) -> &'static [u8] {
    match direction {
        "up" => include_bytes!("../icons/arrow-up-short.svg"),
        &_ => include_bytes!("../icons/arrow-down-short.svg"),
    }
}
