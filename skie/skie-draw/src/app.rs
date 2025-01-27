use std::sync::Arc;
pub use winit;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
pub use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
pub use winit::window::{Window, WindowAttributes};

use crate::{BackendRenderTarget, Canvas, GpuContext, Size};
pub use winit::dpi::{LogicalSize, PhysicalSize};

pub trait SkieAppHandle: 'static {
    fn on_keydown(&mut self, _keycode: KeyCode) {}
    fn on_keyup(&mut self, _keycode: KeyCode) {}
    fn init(&mut self) -> WindowAttributes;
    fn on_create_window(&mut self, _window: &Window) {}
    fn update(&mut self, window: &Window);
    fn draw(&mut self, cx: &mut Canvas, window: &Window);
}

struct App<'a> {
    surface: Option<BackendRenderTarget<'static>>,
    window: Option<Arc<Window>>,
    #[allow(unused)]
    gpu: GpuContext,
    canvas: Canvas,
    app_handle: &'a mut dyn SkieAppHandle,
}

impl<'a> App<'a> {
    async fn new(user_app: &'a mut dyn SkieAppHandle) -> anyhow::Result<Self> {
        let gpu = GpuContext::new().await?;

        let canvas = Canvas::create(Size::default()).build(gpu.clone());

        Ok(Self {
            surface: None,
            window: None,
            gpu,
            canvas,
            app_handle: user_app,
        })
    }
}

pub async fn launch(handle: &mut dyn SkieAppHandle) -> anyhow::Result<()> {
    let mut app = App::new(handle).await?;
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.window.get_or_insert_with(|| {
            let window_attributes = self.app_handle.init();

            let window = event_loop
                .create_window(window_attributes)
                .expect("Error creating window");

            let window = Arc::new(window);

            self.app_handle.on_create_window(&window);

            let size = window.inner_size();
            self.canvas.resize(size.width, size.height);
            let surface = self
                .canvas
                .create_backend_target(window.clone())
                .expect("error creating surface");

            self.surface = Some(surface);

            window
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            self.app_handle.update(window);
            window.request_redraw()
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let window = self.window.as_ref();

        if window.is_none() {
            event_loop.exit();
        }
        let window = window.unwrap();

        match event {
            winit::event::WindowEvent::CloseRequested => {
                self.surface = None;
                self.window = None;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(keycode),
                        state,
                        ..
                    },
                ..
            } => {
                match state {
                    ElementState::Pressed => self.app_handle.on_keydown(keycode),
                    ElementState::Released => {
                        self.app_handle.on_keyup(keycode);
                    }
                };
            }
            WindowEvent::RedrawRequested => {
                if let Some(surface) = &mut self.surface {
                    self.canvas.clear();

                    self.app_handle.draw(&mut self.canvas, window);

                    if let Ok(surface_texture) = self.canvas.paint(surface) {
                        surface_texture.present()
                    } else {
                        eprintln!("Error painting");
                    }
                }
            }
            WindowEvent::Resized(size) => {
                self.canvas.resize(size.width, size.height);
            }
            _ => {}
        }
    }
}
