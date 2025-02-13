pub mod div;
pub use div::*;
pub mod text;
pub use text::*;

use std::cell::RefCell;

use crate::{
    app::AppContext,
    arena::{ArenaAllocator, ArenaElement},
    window::Window,
};

pub trait Element: IntoElement + 'static {
    fn paint(&mut self, window: &mut Window, cx: &mut AppContext);

    fn into_any(self) -> AnyElement {
        AnyElement::new(self)
    }
}

pub trait IntoElement: Sized {
    type Element: Element;

    fn into_element(self) -> Self::Element;
    fn into_any_element(self) -> AnyElement {
        self.into_element().into_any()
    }
}

pub trait ParentElement {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>);
    fn child(mut self, element: impl IntoElement) -> Self
    where
        Self: Sized,
    {
        self.extend(std::iter::once(element.into_element().into_any()));
        self
    }
    fn children(mut self, elements: impl IntoIterator<Item = impl IntoElement>) -> Self
    where
        Self: Sized,
    {
        self.extend(
            elements
                .into_iter()
                .map(|element| element.into_any_element()),
        );
        self
    }
}

pub trait Render: 'static + Sized {
    fn render(&mut self, window: &mut Window, cx: &mut AppContext) -> impl IntoElement;
}

pub trait ElementObject {
    fn paint(&mut self, window: &mut Window, cx: &mut AppContext);
}

struct Paintable<E: Element> {
    element: E,
}

impl<E> ElementObject for Paintable<E>
where
    E: Element,
{
    fn paint(&mut self, window: &mut Window, cx: &mut AppContext) {
        self.element.paint(window, cx)
    }
}

thread_local! {
pub(crate) static ELEMENT_ARENA: RefCell<ArenaAllocator> =
    RefCell::new(ArenaAllocator::new(8 * 1024 * 1024));
}

pub struct AnyElement(ArenaElement<dyn ElementObject>);

impl ElementObject for AnyElement {
    fn paint(&mut self, window: &mut Window, cx: &mut AppContext) {
        self.0.paint(window, cx)
    }
}

impl AnyElement {
    pub fn new<E: Element + 'static>(element: E) -> Self {
        let el = ELEMENT_ARENA
            .with_borrow_mut(|arena| arena.alloc(|| Paintable { element }))
            .map(|el| el as &mut dyn ElementObject);

        Self(el)
    }
}
