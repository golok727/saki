use derive_more::derive::{Deref, DerefMut};
use std::sync::Arc;
pub use winit;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
pub use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
pub use winit::window::{Window, WindowAttributes};

use crate::{
    Canvas, Color, GpuContext, GpuSurface, GpuSurfaceSpecification, GpuTextureViewDescriptor, Size,
};
pub use winit::dpi::{LogicalSize, PhysicalSize};

#[derive(Deref, DerefMut)]
pub struct DrawingContext {
    clear_color: Color,
    #[deref]
    #[deref_mut]
    canvas: Canvas,
}

impl DrawingContext {
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }
}

pub trait SkieAppHandle: 'static {
    fn on_keydown(&mut self, _keycode: KeyCode) {}
    fn on_keyup(&mut self, _keycode: KeyCode) {}
    fn init(&mut self) -> WindowAttributes;
    fn on_create_window(&mut self, _window: &Window) {}
    fn update(&mut self, window: &Window);
    fn draw(&mut self, cx: &mut DrawingContext, window: &Window);
}

struct App<'a> {
    surface: Option<GpuSurface<'static>>,
    window: Option<Arc<Window>>,
    gpu: GpuContext,
    cx: DrawingContext,
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
            cx: DrawingContext {
                canvas,
                clear_color: Color::WHITE,
            },
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
            let surface = self
                .gpu
                .create_surface(
                    window.clone(),
                    &GpuSurfaceSpecification {
                        width: size.width,
                        height: size.height,
                    },
                )
                .expect("error creating surface");

            self.surface = Some(surface);
            self.cx.canvas.resize(size.width, size.height);

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
                    let surface_texture = surface.get_current_texture().unwrap();
                    let view = surface_texture
                        .texture
                        .create_view(&GpuTextureViewDescriptor::default());

                    self.cx.canvas.clear();
                    self.app_handle.draw(&mut self.cx, window);
                    self.cx.canvas.finish(&view, self.cx.clear_color.into());

                    surface_texture.present();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface {
                    surface.resize(&self.gpu, size.width, size.height);
                    self.cx.canvas.resize(size.width, size.height);
                }
            }
            _ => {}
        }
    }
}
