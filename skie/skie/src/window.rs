pub mod error;
use derive_more::derive::{Deref, DerefMut};

use std::{cell::Cell, future::Future, sync::Arc};

use crate::{
    app::{AppContext, AsyncAppContext},
    jobs::Job,
    view::View,
    AnyElement, ElementObject, Render,
};
use anyhow::Result;
use error::CreateWindowError;
pub(crate) use winit::window::Window as WinitWindow;

use skie_draw::{
    gpu, paint::SkieAtlas, BackendRenderTarget, Canvas, Color, GpuContext, TextSystem,
};

#[derive(Debug, Clone)]
pub struct WindowSpecification {
    /// Width of window in logical pixels
    pub width: u32,
    /// Height of window in logical pixels
    pub height: u32,
    /// Title for the window
    pub title: &'static str,
    /// Background color dor with window
    pub background: Color,
}

pub type WindowId = winit::window::WindowId;

impl Default for WindowSpecification {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            title: "skie",
            background: Color::WHITE,
        }
    }
}

impl WindowSpecification {
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }
}

pub struct Window {
    clear_color: Color,

    #[allow(unused)]
    pub(crate) texture_atlas: Arc<SkieAtlas>,

    pub(crate) canvas: Canvas,

    pub(crate) dirty: Cell<bool>,
    root: Option<View>,
    tree: Option<AnyElement>,

    surface: BackendRenderTarget<'static>,

    pub(crate) handle: Arc<WinitWindow>,
}

impl Window {
    pub(crate) fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
        gpu: GpuContext,
        texture_atlas: Arc<SkieAtlas>,
        text_system: Arc<TextSystem>,
    ) -> Result<Self> {
        let width = specs.width;
        let height = specs.height;

        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::LogicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let handle = Arc::new(winit_window);

        let canvas = Canvas::create()
            .width(width)
            .height(height)
            .msaa_samples(4)
            .surface_format(gpu::TextureFormat::Rgba8Unorm)
            .with_text_system(text_system.clone())
            .with_texture_atlas(texture_atlas.clone())
            .build(gpu);

        let surface = canvas.create_backend_target(Arc::clone(&handle))?;

        Ok(Self {
            handle,
            canvas,
            surface,
            dirty: Cell::new(true),
            root: None,
            tree: None,
            texture_atlas,
            clear_color: specs.background,
        })
    }

    pub fn set_bg_color(&mut self, color: Color) {
        self.clear_color = color;
        self.refresh();
    }

    #[inline]
    pub fn id(&self) -> winit::window::WindowId {
        self.handle.id()
    }

    pub(crate) fn handle_resize(&mut self, width: u32, height: u32) {
        self.canvas.resize(width, height);
    }

    pub fn winit_handle(&self) -> &Arc<WinitWindow> {
        &self.handle
    }

    pub fn spawn<Fut, R>(
        &self,
        app: &mut AppContext,
        f: impl FnOnce(AsyncWindowContext) -> Fut,
    ) -> Job<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        app.spawn(|app| {
            f(AsyncWindowContext {
                app,
                window_id: self.id(),
            })
        })
    }

    pub fn mount<V: Render + 'static>(&mut self, view: V) {
        let root = View::new(view);
        self.root.replace(root);
        self.refresh();
    }

    pub fn set_timeout(
        &self,
        app: &mut AppContext,
        f: impl FnOnce(&mut Window, &mut AppContext) + 'static,
        timeout: std::time::Duration,
    ) {
        let window_id = self.id();

        app.set_timeout(
            move |app| {
                let _ = app.update_window(&window_id, |window, app| f(window, app));
            },
            timeout,
        )
    }

    pub(crate) fn paint(&mut self) -> Result<()> {
        self.canvas.clear();
        self.canvas.clear_color(self.clear_color);
        let scale_factor = self.handle.scale_factor() as f32;

        self.canvas.save();
        self.canvas.scale(scale_factor, scale_factor);

        if self.dirty.get() {
            if let Some(mut view) = self.root.take() {
                let tree = view.render(self);
                self.tree.replace(tree);
                self.dirty.set(false);
                self.root.replace(view);
            }
        }

        if let Some(mut root) = self.tree.take() {
            root.paint(self);
            self.tree.replace(root);
        }

        self.canvas.render(&mut self.surface)?.present();
        self.canvas.restore();

        Ok(())
    }

    pub fn refresh(&self) {
        self.handle.request_redraw();
    }
}

#[derive(Deref, DerefMut)]
pub struct AsyncWindowContext {
    #[deref]
    #[deref_mut]
    app: AsyncAppContext,
    window_id: WindowId,
}

impl AsyncWindowContext {
    pub fn update_window<R, Update>(&self, update: Update) -> Result<R>
    where
        Update: FnOnce(&mut Window, &mut AppContext) -> R,
    {
        self.app.update_window(&self.window_id, update)
    }
    #[inline]
    pub fn spawn<Fut, R>(&self, f: impl FnOnce(AsyncWindowContext) -> Fut) -> Job<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        self.app.jobs.spawn(f(Self {
            app: self.app.clone(),
            window_id: self.window_id,
        }))
    }

    #[inline]
    pub fn spawn_blocking<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Job<T>
    where
        T: Send + 'static,
    {
        self.app.jobs.spawn_blocking(future)
    }
}
