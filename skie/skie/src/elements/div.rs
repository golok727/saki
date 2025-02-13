use skie_draw::{Brush, Color, Corners, Rect};

use crate::{app::AppContext, Window};

use super::{AnyElement, Element, ElementObject, IntoElement, ParentElement};

pub struct Div {
    children: Vec<AnyElement>,
    background_color: Color,
}

pub fn div() -> Div {
    Div {
        children: Default::default(),
        background_color: Color::WHITE,
    }
}

impl Div {
    pub fn bg(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }
}

impl Element for Div {
    fn paint(&mut self, window: &mut Window, cx: &mut AppContext) {
        let rect = Rect::xywh(0.0, 0.0, 400.0, 400.0);
        window.canvas.draw_round_rect(
            &rect,
            Corners::with_all(20.0),
            Brush::filled(self.background_color),
        );

        for children in &mut self.children {
            children.paint(window, cx);
        }
    }
}

impl ParentElement for Div {
    fn extend(&mut self, elements: impl IntoIterator<Item = super::AnyElement>) {
        self.children.extend(elements)
    }
}

impl IntoElement for Div {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
