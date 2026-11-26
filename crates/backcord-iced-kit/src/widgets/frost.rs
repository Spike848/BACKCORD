//! Frosted glass under a menu or dialog: [`Frost`] draws the part of a
//! blurred copy of the window's custom background that lies under it, so
//! a see-through panel shows the picture soft behind it. The app hands
//! the copy over with [`set_glass`] as it's made, and where the window
//! shows the picture as it draws.

use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use iced::{
    Element, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget,
        image::Handle,
        layout, mouse, overlay, renderer,
        widget::{Operation, Tree, tree},
    },
};

use super::{Renderer, draw_picture};

/// A blurred copy of the background: its pixels in the renderer's format,
/// its size, and a number that changes with each copy.
pub struct Glass {
    pub pixels: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub generation: u64,
}

/// The copy, the part of it the window shows (in its pixels), and the
/// window's size.
type Shown = (Arc<Glass>, Rectangle, Size);

static GLASS: Mutex<Option<Shown>> = Mutex::new(None);

/// The glass frosted panels show, or none (no background, or no blur).
pub fn set_glass(glass: Option<(Arc<Glass>, Rectangle, Size)>) {
    if let Ok(mut shown) = GLASS.lock() {
        *shown = glass;
    }
}

fn shown() -> Option<Shown> {
    GLASS.lock().ok()?.clone()
}

/// A panel with the glass under it, filling it, its corners cut round as
/// `radius`: it takes the panel's size and passes everything else to it.
pub struct Frost<'a, M> {
    radius: [f32; 4],
    content: Element<'a, M, Theme, Renderer>,
}

impl<'a, M> Frost<'a, M> {
    pub fn new(radius: [f32; 4], content: impl Into<Element<'a, M, Theme, Renderer>>) -> Self {
        Self {
            radius,
            content: content.into(),
        }
    }
}

/// The last cut made, by what it was cut from.
#[derive(Default)]
struct Cut(RefCell<Option<([u32; 4], u64, Handle)>>);

impl<M> Widget<M, Theme, Renderer> for Frost<'_, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Cut>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Cut::default())
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
        if let Some(inner) = layout.children().next() {
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
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
        // Each in a layer of its own, the glass first: in one layer the
        // renderer draws pictures over every quad, the content's switches
        // and grounds too.
        iced::advanced::Renderer::with_layer(renderer, layout.bounds(), |renderer| {
            self.draw_glass(tree, renderer, layout);
        });
        if let Some(inner) = layout.children().next() {
            iced::advanced::Renderer::with_layer(renderer, *viewport, |renderer| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    inner,
                    cursor,
                    viewport,
                );
            });
        }
    }
}

impl<M> Frost<'_, M> {
    fn draw_glass(&self, tree: &Tree, renderer: &mut Renderer, layout: Layout<'_>) {
        let Some((glass, crop, window)) = shown() else {
            return;
        };
        let bounds = layout.bounds();
        if window.width <= 0.0 || window.height <= 0.0 || bounds.width < 1.0 || bounds.height < 1.0
        {
            return;
        }
        let (across, down) = (crop.width / window.width, crop.height / window.height);
        let left = (crop.x + bounds.x * across).clamp(0.0, glass.width as f32 - 1.0);
        let top = (crop.y + bounds.y * down).clamp(0.0, glass.height as f32 - 1.0);
        let right =
            (crop.x + (bounds.x + bounds.width) * across).clamp(left + 1.0, glass.width as f32);
        let bottom =
            (crop.y + (bounds.y + bounds.height) * down).clamp(top + 1.0, glass.height as f32);
        let key = [
            left as u32,
            top as u32,
            (right - left).ceil().max(1.0) as u32,
            (bottom - top).ceil().max(1.0) as u32,
        ];
        let key = [
            key[0],
            key[1],
            key[2].min(glass.width - key[0]),
            key[3].min(glass.height - key[1]),
        ];
        let cut = tree.state.downcast_ref::<Cut>();
        let mut made = cut.0.borrow_mut();
        let fresh = made
            .as_ref()
            .is_some_and(|(at, generation, _)| *at == key && *generation == glass.generation);
        if !fresh {
            let [x, y, width, height] = key;
            let row = glass.width as usize * 4;
            let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
            for line in y..y + height {
                let start = line as usize * row + x as usize * 4;
                pixels.extend_from_slice(&glass.pixels[start..start + width as usize * 4]);
            }
            *made = Some((
                key,
                glass.generation,
                Handle::from_rgba(width, height, pixels),
            ));
        }
        if let Some((_, _, handle)) = made.as_ref() {
            // A corner no rounder than half the side, or the path can't close.
            let most = bounds.width.min(bounds.height) / 2.0 - 0.5;
            let radius = self.radius.map(|radius| radius.clamp(0.0, most.max(0.0)));
            draw_picture(renderer, handle, bounds, radius, 1.0);
        }
    }
}

impl<'a, M: 'a> From<Frost<'a, M>> for Element<'a, M, Theme, Renderer> {
    fn from(frost: Frost<'a, M>) -> Self {
        Element::new(frost)
    }
}
