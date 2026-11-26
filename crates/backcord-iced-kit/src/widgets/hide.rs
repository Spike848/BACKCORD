//! What's drawn only while it's shown: hidden, it keeps its place, its
//! state and everything it hears, and draws nothing. The window hides
//! what's open over it for the one frame its menus' glass is taken.

use iced::{
    Element, Length, Rectangle, Size, Theme, Vector,
    advanced::{
        Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
        widget::{Operation, Tree},
    },
};

use super::Renderer;

/// `content`, drawn unless `hidden`.
pub struct Hide<'a, M> {
    content: Element<'a, M, Theme, Renderer>,
    hidden: bool,
}

impl<'a, M> Hide<'a, M> {
    pub fn new(content: impl Into<Element<'a, M, Theme, Renderer>>, hidden: bool) -> Self {
        Self {
            content: content.into(),
            hidden,
        }
    }
}

impl<M> Widget<M, Theme, Renderer> for Hide<'_, M> {
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
        if self.hidden {
            return;
        }
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
        let overlay = self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            inner,
            renderer,
            viewport,
            translation,
        )?;
        // Hidden, it's still there, and hears what it hears: only not drawn.
        Some(if self.hidden {
            overlay::Element::new(Box::new(Hidden(overlay)))
        } else {
            overlay
        })
    }
}

/// An overlay that keeps its place and what it hears and draws nothing.
struct Hidden<'a, M>(overlay::Element<'a, M, Theme, Renderer>);

impl<M> overlay::Overlay<M, Theme, Renderer> for Hidden<'_, M> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        self.0.as_overlay_mut().layout(renderer, bounds)
    }

    fn draw(
        &self,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
    }

    fn operate(&mut self, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.0.as_overlay_mut().operate(layout, renderer, operation);
    }

    fn update(
        &mut self,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
    ) {
        self.0
            .as_overlay_mut()
            .update(event, layout, cursor, renderer, clipboard, shell);
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.0
            .as_overlay()
            .mouse_interaction(layout, cursor, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        layout: Layout<'b>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, M, Theme, Renderer>> {
        let nested = self.0.as_overlay_mut().overlay(layout, renderer)?;
        Some(overlay::Element::new(Box::new(Hidden(nested))))
    }

    fn index(&self) -> f32 {
        self.0.as_overlay().index()
    }
}

impl<'a, M: 'a> From<Hide<'a, M>> for Element<'a, M, Theme, Renderer> {
    fn from(hide: Hide<'a, M>) -> Self {
        Element::new(hide)
    }
}
