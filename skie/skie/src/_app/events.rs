use std::{cell::RefCell, sync::Arc};

use parking_lot::Mutex;

use winit::event_loop::EventLoopProxy;

use crate::window::{WindowContext, WindowId};

use super::{AppAction, AppContext, AppUpdateEvent};

#[derive(Default, Clone)]
pub struct AppEvents(Arc<Mutex<AppEventsState>>);

#[derive(Default)]
struct AppEventsState {
    app_events: RefCell<Vec<AppUpdateEvent>>,
    proxy: Option<EventLoopProxy<AppAction>>,
}

impl AppEvents {
    pub fn init(&self, proxy: EventLoopProxy<AppAction>) {
        let mut lock = self.0.lock();
        lock.proxy = Some(proxy);
    }

    pub fn notify(&self, event: AppAction) {
        let lock = self.0.lock();
        if let Some(proxy) = &lock.proxy {
            let _ = proxy.send_event(event);
        }
    }

    pub fn push_event(&self, ev: AppUpdateEvent) {
        let lock = self.0.lock();
        RefCell::borrow_mut(&lock.app_events).push(ev);
    }

    pub fn app_context_callback(&self, f: impl FnOnce(&mut AppContext) + 'static) {
        self.push_event(AppUpdateEvent::AppContextCallback {
            callback: Box::new(f),
        });
        self.notify(AppAction::AppUpdate);
    }

    #[allow(unused)]
    pub fn window_context_callback(
        &self,
        window_id: WindowId,
        f: impl FnOnce(&mut WindowContext) + 'static,
    ) {
        self.push_event(AppUpdateEvent::WindowContextCallback {
            callback: Box::new(f),
            window_id,
        });
        self.notify(AppAction::AppUpdate);
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
