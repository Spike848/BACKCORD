//! A selection box dragged over a list: pressed where nothing under the
//! pointer takes the press (a list's blank space or its margins), it grows
//! with the pointer, drawn see-through in the accent, and says where it is
//! as it changes and once more as it's let go, in the list's own terms
//! (its top left 0, 0, not counting how far it's scrolled).

use iced::{
    Border, Color, Element, Length, Point, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};

use super::Renderer;

/// How far the pointer moves before a press becomes a box.
const SLOP: f32 = 4.0;

/// `content` with a selection box over it.
pub struct Marquee<'a, M> {
    content: Element<'a, M, Theme, Renderer>,
    on_change: Box<dyn Fn(Rectangle, bool) -> M + 'a>,
    ink: Color,
}

impl<'a, M> Marquee<'a, M> {
    /// `on_change` hears the box, and whether it was let go of.
    pub fn new(
        content: impl Into<Element<'a, M, Theme, Renderer>>,
        ink: Color,
        on_change: impl Fn(Rectangle, bool) -> M + 'a,
    ) -> Self {
        Self {
            content: content.into(),
            on_change: Box::new(on_change),
            ink,
        }
    }
}

/// Where a press started, and the pointer since, while one is held.
#[derive(Default)]
struct Held {
    from: Option<Point>,
    to: Point,
    dragging: bool,
}

fn boxed(from: Point, to: Point) -> Rectangle {
    let (x, y) = (from.x.min(to.x), from.y.min(to.y));
    Rectangle::new(
        Point::new(x, y),
        Size::new((from.x - to.x).abs(), (from.y - to.y).abs()),
    )
}

impl<M> Widget<M, Theme, Renderer> for Marquee<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Held>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Held::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let inner = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        layout::Node::with_children(inner.size(), vec![inner])
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(inner) = layout.children().next() {
            self.content
                .as_widget_mut()
                .operate(&mut tree.children[0], inner, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let held = tree.state.downcast_mut::<Held>();
        // While a box is being drawn, the list under it hears nothing.
        if !held.dragging
            && let Some(inner) = layout.children().next()
        {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                inner,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
        let held = tree.state.downcast_mut::<Held>();
        let local = |point: Point| Point::new(point.x - bounds.x, point.y - bounds.y);
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if !shell.is_event_captured()
                    && let Some(at) = cursor.position_over(bounds)
                {
                    *held = Held {
                        from: Some(local(at)),
                        to: local(at),
                        dragging: false,
                    };
                }
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(from) = held.from {
                    held.to = local(*position);
                    let moved = (held.to - from) * 1.0;
                    if !held.dragging && moved.x.abs().max(moved.y.abs()) > SLOP {
                        held.dragging = true;
                    }
                    if held.dragging {
                        shell.publish((self.on_change)(boxed(from, held.to), false));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if let Some(from) = held.from.take() {
                    if held.dragging {
                        shell.publish((self.on_change)(boxed(from, held.to), true));
                        shell.request_redraw();
                        shell.capture_event();
                    }
                    held.dragging = false;
                }
            }
            _ => {}
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
        if tree.state.downcast_ref::<Held>().dragging {
            return mouse::Interaction::Crosshair;
        }
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |inner| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    inner,
                    cursor,
                    viewport,
                    renderer,
                )
            })
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
        use iced::advanced::Renderer as _;
        if let Some(inner) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                inner,
                cursor,
                viewport,
            );
        }
        let held = tree.state.downcast_ref::<Held>();
        let (Some(from), true) = (held.from, held.dragging) else {
            return;
        };
        let bounds = layout.bounds();
        let shown = boxed(from, held.to) + Vector::new(bounds.x, bounds.y);
        let Some(shown) = shown.intersection(&bounds) else {
            return;
        };
        let ink = self.ink;
        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: shown,
                    border: Border {
                        color: ink,
                        width: 1.0,
                        radius: 2.0.into(),
                    },
                    ..renderer::Quad::default()
                },
                Color { a: 0.18, ..ink },
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let inner = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            inner,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<Marquee<'a, M>> for Element<'a, M, Theme, Renderer> {
    fn from(marquee: Marquee<'a, M>) -> Self {
        Element::new(marquee)
    }
}
