use std::rc::Weak;

use winit::event_loop::ActiveEventLoop;

use crate::jobs::Jobs;

use super::{AppAction, AppContextCell};

#[derive(Clone)]
pub struct AsyncAppContext {
    pub(crate) app: Weak<AppContextCell>,
    pub(crate) jobs: Jobs,
}

impl AsyncAppContext {
    pub(super) fn handle_on_resumed(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let cx = self.app.upgrade().expect("app  released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_resumed(event_loop);
    }

    pub(super) fn handle_window_event(
        &self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_window_event(event_loop, window_id, event);
    }

    pub(super) fn handle_on_about_to_wait(&self, event_loop: &ActiveEventLoop) {
        // If we put this inside the context.handle_on_about_to_wait it will cause a double borrow
        self.jobs.run_foregound_tasks();

        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_about_to_wait(event_loop);
    }

    pub(super) fn handle_on_user_event(&self, event_loop: &ActiveEventLoop, event: AppAction) {
        let cx = self.app.upgrade().expect("app released");
        let mut lock = cx.borrow_mut();
        lock.handle_on_user_event(event_loop, event);
    }
}
