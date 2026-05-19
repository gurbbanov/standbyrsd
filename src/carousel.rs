use crate::message::Message;
use crate::{IDLE_MS, SNAP_DURATION_MS, SNAP_THRESHOLD};
use iced::advanced::{
    Clipboard, Renderer as AdvancedRenderer, Shell,
    layout::{Layout, Limits, Node},
    renderer,
    widget::{Tree, tree},
};
use iced::{Element, Length, Rectangle, Renderer, Size, Theme, Vector, mouse};
use std::time::Instant;

struct VerticalCarousel<'a> {
    items: Vec<Element<'a, Message>>,
    slot_width: f32,
    slot_height: f32,
    initial_current: usize,
    on_change: Box<dyn Fn(usize) -> Message + 'a>,
}

pub fn vertical_carousel<'a>(
    items: Vec<Element<'a, Message>>,
    slot_width: f32,
    slot_height: f32,
    initial_current: usize,
    on_change: impl Fn(usize) -> Message + 'a,
) -> Element<'a, Message> {
    VerticalCarousel {
        items,
        slot_width,
        slot_height,
        initial_current,
        on_change: Box::new(on_change),
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

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<CarouselState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(CarouselState {
            current: self.initial_current,
            ..Default::default()
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.items.iter().map(|c| Tree::new(c)).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.items);
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let sw = self.slot_width;
        let sh = self.slot_height;
        let child_limits = Limits::new(Size::ZERO, Size::new(sw, sh));

        let children: Vec<Node> = self
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

        Node::with_children(
            limits.resolve(Length::Fixed(sw), Length::Fixed(sh), Size::new(sw, sh)),
            children,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
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
        tree: &mut Tree,
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
                        let prev = state.current;
                        state.try_snap(count, sh);
                        if state.current != prev {
                            shell.publish((self.on_change)(state.current));
                        }
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
                            let prev = state.current;
                            state.try_snap(count, sh);
                            if state.current != prev {
                                shell.publish((self.on_change)(state.current));
                            }
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
        tree: &Tree,
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

#[derive(Debug, Clone, Copy)]
pub enum CarouselId {
    Page0Left,
    Page0Right,
    Page1,
}

pub fn ease_spring(t: f32, v0: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    let hermite = 3.0 * t2 - 2.0 * t3;
    let velocity_term = v0 * t * (t - 1.0) * (t - 1.0);
    (hermite + velocity_term).clamp(0.0, 1.0)
}
