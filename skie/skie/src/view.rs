use crate::app::world::{AnyEntity, Entity};
use crate::app::AppContext;
use crate::elements::AnyElement;
use crate::{Element, ElementObject, IntoElement, Render, Window};

#[derive(Clone, Debug)]
pub struct View {
    handle: AnyEntity,
    render: fn(&View, &mut Window, cx: &mut AppContext) -> AnyElement,
}

impl View {
    pub fn downcast<T: 'static>(self) -> Result<Entity<T>, Self> {
        match self.handle.downcast() {
            Ok(handle) => Ok(handle),
            Err(handle) => Err(Self {
                handle,
                render: self.render,
            }),
        }
    }
}

impl<T: Render + 'static> From<Entity<T>> for View {
    fn from(value: Entity<T>) -> Self {
        Self {
            handle: value.into_any(),
            render: render_view::<T>,
        }
    }
}

impl<E: Render + 'static> Element for Entity<E> {
    fn paint(&mut self, window: &mut Window, cx: &mut crate::app::AppContext) {
        let mut elem = self.update(cx, |view, cx| view.render(window, cx).into_any_element());
        elem.paint(window, cx);
    }
}

impl<E: Render + 'static> IntoElement for Entity<E> {
    type Element = Entity<E>;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for View {
    fn paint(&mut self, window: &mut Window, cx: &mut crate::app::AppContext) {
        let mut element = (self.render)(self, window, cx);
        element.paint(window, cx);
    }
}

impl IntoElement for View {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn render_view<T: Render + 'static>(
    view: &View,
    window: &mut Window,
    cx: &mut AppContext,
) -> AnyElement {
    let view = view.clone().downcast::<T>().unwrap();
    view.update(cx, |view, cx| view.render(window, cx).into_any_element())
}
