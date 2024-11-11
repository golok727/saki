use std::sync::Arc;

use saki_draw::Renderer;
use winit::{application::ApplicationHandler, event::WindowEvent, window::WindowAttributes};

type InitCallback = Option<Box<dyn FnOnce(&mut App)>>;

struct App {
    window: Option<Arc<winit::window::Window>>,
    init_callback: InitCallback,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            init_callback: None,
        }
    }

    pub fn run<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App) + 'static,
    {
        self.init_callback = Some(Box::new(f));
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        if let Err(err) = event_loop.run_app(self) {
            println!("Error running app: Error: {}", err);
        };

        let renderer = Renderer {};
        renderer.render();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(WindowAttributes::default())
                .unwrap();
            self.window = Some(Arc::new(window));

            if let Some(cb) = self.init_callback.take() {
                cb(self);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(window) = &self.window {
                    if window.id() == window_id {
                        event_loop.exit();
                    }
                }
            }
            _ => {
                //
            }
        }
    }
}

fn main() {
    println!("Radhe Shyam!");

    let mut app = App::new();

    app.run(|_| {
        let renderer = Renderer::default();
        renderer.render();
    });
}
