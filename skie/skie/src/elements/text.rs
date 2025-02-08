use skie_draw::Color;

use super::{Element, IntoElement};

pub struct TextElement {
    text: &'static str,
}

impl Element for TextElement {
    fn paint(&mut self, window: &mut crate::Window) {
        window.canvas.fill_text(
            &skie_draw::Text::new(self.text)
                .pos(20.0, 20.0)
                .size_px(24.0),
            Color::BLACK,
        )
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[inline]
pub fn text(text: &'static str) -> TextElement {
    TextElement { text }
}

impl IntoElement for &'static str {
    type Element = TextElement;

    fn into_element(self) -> Self::Element {
        TextElement { text: self }
    }
}
