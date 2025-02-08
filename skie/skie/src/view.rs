use crate::elements::AnyElement;
use crate::{IntoElement, Render, Window};

pub struct View {
    // will be changed after managing objects directly in the app
    render: Box<dyn FnMut(&mut Window) -> AnyElement>,
}

impl View {
    pub fn new<V: Render + 'static>(mut view: V) -> Self {
        Self {
            render: Box::new(move |window| view.render(window).into_any_element()),
        }
    }

    pub fn render(&mut self, window: &mut Window) -> AnyElement {
        (self.render)(window)
    }
}
