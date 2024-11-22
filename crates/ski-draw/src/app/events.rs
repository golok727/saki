use std::{cell::RefCell, rc::Rc, sync::Arc};

use winit::event_loop::EventLoopProxy;

use super::{AppAction, AppUpdateEvent};

#[derive(Default, Clone)]
pub struct AppEvents(Arc<AppEventsState>);

impl AppEvents {
    pub fn send() {}
}

#[derive(Default)]
struct AppEventsState {
    app_events: Rc<RefCell<Vec<AppUpdateEvent>>>,
    proxy: Option<EventLoopProxy<AppAction>>,
}

impl AppEvents {}
