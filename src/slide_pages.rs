use crate::SNAP_DURATION_MS;
use crate::message::Message;
use iced::advanced::{
    Clipboard, Renderer as AdvanedRenderer, Shell,
    layout::{Layout, Limits, Node},
    renderer,
    widget::Tree,
};
use iced::{Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum DragState {
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
    pub fn is_snapping_done(&self) -> bool {
        if let DragState::Snapping { started_at, .. } = self {
            started_at.elapsed().as_millis() >= SNAP_DURATION_MS as u128
        } else {
            false
        }
    }
}

pub struct SlidePages<'a, M, T, R> {
    offset: f32,
    page_width: f32,
    page_height: f32,
    children: Vec<Element<'a, M, T, R>>,
}

pub fn slide_pages_func<'a>(
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

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let pw = self.page_width;
        let ph = self.page_height;
        let child_limits = Limits::new(Size::ZERO, Size::new(pw, ph));

        let children: Vec<Node> = self
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

        Node::with_children(
            limits.resolve(Length::Fill, Length::Fill, Size::new(pw, ph)),
            children,
        )
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(|c| Tree::new(c)).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
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

        if bounds.width > 0.0 {
            shell.publish(Message::UpdatePageWidth(bounds.width));
        }

        if let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
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
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let translated_cursor = match cursor {
            mouse::Cursor::Available(pos) => {
                mouse::Cursor::Available(Point::new(pos.x - self.offset, pos.y))
            }
            other => other,
        };

        self.children
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
