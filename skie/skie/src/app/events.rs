use std::{cell::RefCell, sync::Arc};

use parking_lot::Mutex;

use winit::event_loop::EventLoopProxy;

use super::{AppUpdateEvent, UserEvent};

#[derive(Default, Clone)]
pub(crate) struct AppEvents(Arc<Mutex<AppEventsState>>);

#[derive(Default)]
struct AppEventsState {
    app_events: RefCell<Vec<AppUpdateEvent>>,
    proxy: Option<EventLoopProxy<UserEvent>>,
}

impl AppEvents {
    pub fn init(&self, proxy: EventLoopProxy<UserEvent>) {
        let mut lock = self.0.lock();
        lock.proxy = Some(proxy);
    }

    pub fn notify(&self, event: UserEvent) {
        let lock = self.0.lock();
        if let Some(proxy) = &lock.proxy {
            let _ = proxy.send_event(event);
        }
    }

    pub(crate) fn push_event(&self, ev: AppUpdateEvent) {
        let lock = self.0.lock();
        RefCell::borrow_mut(&lock.app_events).push(ev);
    }

    pub fn dispose(&self) {
        let mut lock = self.0.lock();
        lock.proxy = None;
    }

    pub fn drain(&self) -> Vec<AppUpdateEvent> {
        let lock = self.0.lock();
        lock.app_events.take()
    }
}
