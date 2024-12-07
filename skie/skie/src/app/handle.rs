use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop};

use super::AppAction;

type ResumedCallback = Box<dyn Fn(&ActiveEventLoop)>;
type UserEventCallback = Box<dyn Fn(&ActiveEventLoop, AppAction)>;
type AboutToWaitCallback = Box<dyn Fn(&ActiveEventLoop)>;
type WindowEventCallback = Box<
    dyn Fn(&winit::event_loop::ActiveEventLoop, winit::window::WindowId, winit::event::WindowEvent),
>;

#[derive(Default)]
pub struct AppHandleCallbacks {
    resumed: Option<ResumedCallback>,
    window_event: Option<WindowEventCallback>,
    about_to_wait: Option<AboutToWaitCallback>,
    user_event: Option<UserEventCallback>,
}

#[derive(Default)]
pub struct AppHandle {
    callbacks: AppHandleCallbacks,
}

impl AppHandle {
    pub fn on_resumed(&mut self, callback: ResumedCallback) {
        self.callbacks.resumed = Some(callback);
    }
    pub fn on_window_event(&mut self, callback: WindowEventCallback) {
        self.callbacks.window_event = Some(callback);
    }

    pub fn on_user_event(&mut self, callback: UserEventCallback) {
        self.callbacks.user_event = Some(callback);
    }

    pub fn on_about_to_wait(&mut self, callback: AboutToWaitCallback) {
        self.callbacks.about_to_wait = Some(callback);
    }
}

impl ApplicationHandler<AppAction> for AppHandle {
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(callback) = &self.callbacks.about_to_wait {
            callback(event_loop)
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppAction) {
        if let Some(callback) = &self.callbacks.user_event {
            callback(event_loop, event)
        }
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(callback) = &self.callbacks.resumed {
            callback(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(callback) = &self.callbacks.window_event {
            callback(event_loop, window_id, event)
        }
    }
}
