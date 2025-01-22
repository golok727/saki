pub mod error;
use parking_lot::RwLock;

use core::f32;
use std::{borrow::Cow, future::Future, io::Read, path::Path, sync::Arc};

use crate::{
    app::{AppContext, AsyncAppContext},
    jobs::Job,
    Pixels,
};
use anyhow::{anyhow, Result};
use error::CreateWindowError;
use image::{ImageBuffer, RgbaImage};
pub(crate) use winit::window::Window as WinitWindow;

use skie_draw::{
    circle,
    paint::{AsPrimitive, AtlasKey, SkieAtlas, SkieImage, TextureKind},
    quad,
    traits::Half,
    vec2, Color, Corners, Painter, Path2D, Rect, Scene, Size, StrokeStyle, Text, TextureFilterMode,
    TextureId, TextureOptions, Vec2, WgpuRendererSpecs,
};

#[derive(Debug, Clone)]
pub struct WindowSpecification {
    pub width: u32,
    pub height: u32,
    pub title: &'static str,
}

pub type WindowId = winit::window::WindowId;

impl Default for WindowSpecification {
    fn default() -> Self {
        Self {
            width: 800,
            height: 800,
            title: "skie",
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

#[derive(Debug, Clone)]
pub struct ImageObject {
    pub bbox: Rect<Pixels>,
    natural_width: f32,
    natural_height: f32,
    texture: TextureId,
}
impl ImageObject {
    pub fn natural_height(&self) -> f32 {
        self.natural_height
    }

    pub fn natural_width(&self) -> f32 {
        self.natural_width
    }
}

// for playing around remove it later
#[derive(Debug, Clone)]
pub enum Object {
    Image(ImageObject),
}

impl Object {
    pub fn as_image(&self) -> Option<&ImageObject> {
        match self {
            Object::Image(img) => Some(img),
        }
    }

    pub fn as_image_mut(&mut self) -> Option<&mut ImageObject> {
        match self {
            Object::Image(ref mut img) => Some(img),
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    // TODO: active
    mouse_pos: Option<Vec2<f32>>,
}

impl State {
    pub fn set_mouse_pos(&mut self, pos: Vec2<f32>) {
        self.mouse_pos = Some(pos)
    }

    pub fn mouse_pos(&self) -> Option<&Vec2<f32>> {
        self.mouse_pos.as_ref()
    }
}

pub struct Window {
    pub(crate) painter: Painter,
    pub(crate) state: RwLock<State>,

    objects: Vec<Object>,
    clear_color: Color,

    yellow_thing_texture_id: TextureId,
    checker_texture_id: TextureId,

    pub(crate) texture_atlas: Arc<SkieAtlas>,
    next_texture_id: usize,

    scroller: Scroller,

    pub(crate) handle: Arc<WinitWindow>,
}

impl Window {
    pub(crate) fn new(
        app: &AppContext,
        event_loop: &winit::event_loop::ActiveEventLoop,
        specs: &WindowSpecification,
    ) -> Result<Self> {
        let width = specs.width;
        let height = specs.height;

        let attr = winit::window::WindowAttributes::default()
            .with_inner_size(winit::dpi::PhysicalSize::new(width, height))
            .with_title(specs.title);

        let winit_window = event_loop.create_window(attr).map_err(CreateWindowError)?;
        let handle = Arc::new(winit_window);

        let texture_atlas = app.texture_atlas.clone();

        let mut painter = Painter::new(
            app.gpu.clone(),
            Arc::clone(&handle),
            texture_atlas.clone(),
            app.text_system.clone(),
            &(WgpuRendererSpecs { width, height }),
        )?;

        let checker_texture_key = AtlasKey::from(SkieImage::new(1));
        let yellow_thing_texture_key = AtlasKey::from(SkieImage::new(2));

        let checker_data = Cow::Owned(create_checker_texture(250, 250, 25));

        texture_atlas.get_or_insert(&checker_texture_key, || {
            (
                TextureKind::Color,
                Size {
                    width: 250,
                    height: 250,
                },
                checker_data,
            )
        });

        let thing_data = load_thing();
        texture_atlas.get_or_insert(&yellow_thing_texture_key, || {
            (
                TextureKind::Color,
                Size {
                    width: thing_data.width() as _,
                    height: thing_data.height() as _,
                },
                Cow::Borrowed(&thing_data),
            )
        });

        let opts = TextureOptions::default()
            .min_filter(TextureFilterMode::Linear)
            .mag_filter(TextureFilterMode::Linear);

        painter
            .renderer
            .set_texture_from_atlas(&texture_atlas, &checker_texture_key, &opts);

        painter
            .renderer
            .set_texture_from_atlas(&texture_atlas, &yellow_thing_texture_key, &opts);

        let scroller = {
            let size = handle.inner_size();
            let size = Size {
                width: size.width as f32,
                height: size.height as f32,
            };

            let mut dims = Rect::new(size.width.half(), size.height.half(), 500.0, 500.0);
            dims.origin.x -= dims.size.width.half();
            dims.origin.y -= dims.size.height.half();

            Scroller::new(dims)
        };

        Ok(Self {
            handle,
            painter,
            state: RwLock::new(State::default()),
            texture_atlas,
            yellow_thing_texture_id: yellow_thing_texture_key.into(),
            checker_texture_id: checker_texture_key.into(),
            objects: Vec::new(),
            clear_color: Color::WHITE,
            scroller,

            // FIXME: this is bad
            next_texture_id: 10000,
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
        self.painter.resize(width, height);
    }

    pub fn winit_handle(&self) -> &Arc<WinitWindow> {
        &self.handle
    }

    // FIXME: for now
    pub fn build_scene(&mut self) {
        let size = self.winit_handle().inner_size();
        let width = size.width as f32;
        let height = size.height as f32;

        let mut scene = Scene::default();

        scene.add(
            quad()
                .pos(width / 2.0 - 350.0, height / 2.0 - 350.0)
                .size(700.0, 700.0)
                .primitive()
                .textured(&self.yellow_thing_texture_id),
        );

        scene.add(
            quad()
                .pos(100.0, height - 400.0)
                .size(300.0, 300.0)
                .primitive()
                .textured(&self.checker_texture_id)
                .fill_color(Color::from_rgb(0xFF0000)),
        );

        scene.add(
            quad()
                .pos(100.0, 200.0)
                .size(250.0, 250.0)
                .primitive()
                .textured(&self.checker_texture_id)
                .fill_color(Color::from_rgb(0xFFFF00)),
        );

        scene.add(
            quad()
                .pos(width - 300.0, height - 300.0)
                .size(200.0, 200.0)
                .primitive()
                .textured(&self.yellow_thing_texture_id)
                .stroke_width(10),
        );

        scene.add(
            quad()
                .pos(100.0, 500.0)
                .size(300.0, 100.0)
                .primitive()
                .fill_color(Color::from_rgb(0x55a09e)),
        );

        scene.add(
            circle()
                .pos(400.0, 500.0)
                .radius(300.0)
                .primitive()
                .fill_color(Color::KHAKI),
        );

        scene.add(
            circle()
                .pos(400.0, 500.0)
                .radius(200.0)
                .primitive()
                .textured(&self.checker_texture_id)
                .fill_color(Color::TORCH_RED),
        );

        let bar_height: f32 = 50.0;
        let margin_bottom: f32 = 30.0;

        scene.add(
            quad()
                .pos(0.0, height - bar_height - margin_bottom)
                .size(width, bar_height)
                .primitive()
                .fill_color(Color::from_rgb(0x0A0A11)),
        );

        for object in &self.objects {
            match object {
                Object::Image(ImageObject {
                    bbox,
                    texture,
                    natural_width,
                    natural_height,
                }) => {
                    let aspect = natural_width / natural_height;
                    let x: f32 = bbox.origin.x.into();
                    let y: f32 = bbox.origin.y.into();
                    let width: f32 = (bbox.size.width * aspect).into();
                    let height: f32 = (bbox.size.height).into();

                    scene.add(
                        quad()
                            .pos(x, y)
                            .size(width, height)
                            .corners(Corners::with_all(width.half() * 0.2))
                            .primitive()
                            .textured(texture),
                    );
                }
            }
        }

        scene.add(
            quad()
                .pos(800.0, 200.0)
                .size(200.0, 500.0)
                .corners(Corners::with_all(100.0).with_top_left(50.0))
                .primitive()
                .fill_color(Color::LIGHT_GREEN)
                .stroke_width(20)
                .stroke_color(Color::TORCH_RED),
        );

        scene.add(
            quad()
                .pos(width - 200.0, 50.0)
                .size(100.0, 50.0)
                .corners(Corners::with_all(10.0))
                .primitive()
                .fill_color(Color::TORCH_RED)
                .stroke_width(5),
        );

        let mut path = Path2D::default();
        path.move_to((100.0, 100.0).into());
        path.line_to((500.0, 100.0).into());
        path.line_to((100.0, 400.0).into());
        path.close();

        scene.add(
            path.primitive()
                .stroke(StrokeStyle::default().round_join().line_width(50)),
        );

        let mut path = Path2D::default();
        path.move_to((300.0, 500.0).into());
        path.line_to((600.0, 500.0).into());
        path.line_to((400.0, 700.0).into());

        scene.add(
            path.primitive().stroke(
                StrokeStyle::default()
                    .color(Color::WHITE)
                    .round_cap()
                    .bevel_join()
                    .line_width(30),
            ),
        );

        self.painter.paint_scene(&scene);
        {
            let state = self.state.read();
            self.scroller.render(&mut self.painter, state.mouse_pos());
        }

        self.painter
            .draw_text(&Text::new("Radha").size_px(64.0), Color::ORANGE);
    }

    pub(crate) fn handle_scroll_wheel(&mut self, _dx: f32, dy: f32) {
        {
            let state = self.state.read();
            if let Some(pos) = state.mouse_pos() {
                let contains = self.scroller.dims.contains(pos);
                if contains {
                    let something = (10.0 * 10.0 * 10.0) * 0.05 * dy;
                    self.scroller.scroll_x += something;
                    // FIXME: notify app to redraw
                    self.winit_handle().request_redraw();
                }
            }
        }
    }

    pub(crate) fn paint(&mut self) {
        self.painter.begin_frame();
        self.build_scene();
        self.painter.finish(self.clear_color.into());
    }

    fn get_next_tex_id(&mut self) -> usize {
        let id = self.next_texture_id;
        self.next_texture_id += 1;
        id
    }

    // TODO: pub(crate)
    pub fn refresh(&self) {
        self.handle.request_redraw();
    }
}

pub struct AsyncWindowContext {
    app: AsyncAppContext,
    window_id: WindowId,
}

impl AsyncWindowContext {
    pub fn with<R>(&self, reader: impl FnOnce(&mut WindowContext) -> R) -> Option<R> {
        let app = self.app.app.upgrade().expect("app released");
        let mut lock = app.borrow_mut();
        let window = lock.windows.remove(&self.window_id);

        if let Some(mut window) = window {
            let mut cx = WindowContext::new(&mut lock, &mut window);
            let res = reader(&mut cx);
            lock.windows.insert(window.id(), window);
            Some(res)
        } else {
            log::info!("Window was released");
            None
        }
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

    // TODO: replace with assets
    pub async fn load_image_from_file(
        &self,
        bounds: Rect<Pixels>,
        file_path: String,
    ) -> Result<usize> {
        let img_job: Job<Result<_>> =
            self.spawn_blocking(load_image_from_file_async(file_path.clone()));

        self.spawn(|cx| async move {
            let img = img_job.await?;
            cx.with(|cx| {
                let idx =
                    cx.add_image_from_data(&img, Size::new(img.width(), img.height()), bounds);

                // FIXME: mark window as dirty and notify app to redraw instead
                cx.window.refresh();
                idx
            })
            .ok_or(anyhow!("Window was released"))
        })
        .await
    }
}

pub struct WindowContext<'a> {
    pub app: &'a mut AppContext,
    pub window: &'a mut Window,
}

impl<'a> WindowContext<'a> {
    pub fn new(app: &'a mut AppContext, window: &'a mut Window) -> Self {
        Self { app, window }
    }

    pub fn open_window<F>(&mut self, specs: WindowSpecification, f: F)
    where
        F: Fn(&mut WindowContext) + 'static,
    {
        self.app.open_window(specs, f)
    }

    pub fn to_async(&self) -> AsyncWindowContext {
        AsyncWindowContext {
            app: self.app.to_async(),
            window_id: self.window.id(),
        }
    }

    pub fn get_object(&self, index: usize) -> Option<&Object> {
        self.window.objects.get(index)
    }

    pub fn get_object_mut(&mut self, index: usize) -> Option<&mut Object> {
        self.window.objects.get_mut(index)
    }

    fn add_image_from_data(
        &mut self,
        image: &[u8],
        natutal_size: Size<u32>,
        bounds: Rect<Pixels>,
    ) -> usize {
        let width = natutal_size.width;
        let height = natutal_size.height;
        let key = AtlasKey::from(SkieImage::new(self.window.get_next_tex_id()));
        self.window.texture_atlas.get_or_insert(&key, || {
            (
                TextureKind::Color,
                Size {
                    width: width as _,
                    height: height as _,
                },
                Cow::Borrowed(image),
            )
        });

        self.window.painter.renderer.set_texture_from_atlas(
            &self.window.texture_atlas,
            &key,
            &TextureOptions::default()
                .min_filter(TextureFilterMode::Linear)
                .mag_filter(TextureFilterMode::Linear),
        );

        let idx = self.window.objects.len();
        self.window.objects.push(Object::Image(ImageObject {
            bbox: bounds,
            natural_width: width as f32,
            natural_height: height as f32,
            texture: key.into(),
        }));
        idx
    }

    pub fn spawn<Fut, R>(&self, f: impl FnOnce(AsyncWindowContext) -> Fut) -> Job<R>
    where
        Fut: Future<Output = R> + 'static,
        R: 'static,
    {
        let cx = self.to_async();
        self.app.jobs.spawn(f(cx))
    }

    pub fn set_timeout(
        &mut self,
        f: impl FnOnce(&mut WindowContext) + 'static,
        timeout: std::time::Duration,
    ) {
        self.spawn(|cx| async move {
            cx.app.jobs.timer(timeout).await;
            cx.with(|cx| f(cx));
        })
        .detach();
    }
}

fn create_checker_texture(width: usize, height: usize, tile_size: usize) -> Vec<u8> {
    let mut texture_data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_black = (tile_x + tile_y) % 2 == 0;

            let offset = (y * width + x) * 4;
            if is_black {
                texture_data[offset] = 0; // Red
                texture_data[offset + 1] = 0; // Green
                texture_data[offset + 2] = 0; // Blue
                texture_data[offset + 3] = 255; // Alpha
            } else {
                texture_data[offset] = 255; // Red
                texture_data[offset + 1] = 255; // Green
                texture_data[offset + 2] = 255; // Blue
                texture_data[offset + 3] = 255; // Alpha
            }
        }
    }
    texture_data
}

fn load_thing() -> ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    let thing_buffer = include_bytes!("../../../assets/thing2.png");

    let thing = image::load_from_memory(thing_buffer).unwrap();
    thing.into_rgba8()
}

#[derive(Debug)]
struct Scroller {
    scroll_x: f32,
    dims: Rect<f32>,
}

impl Scroller {
    fn new(dims: Rect<f32>) -> Self {
        Self {
            dims,
            scroll_x: 0.0,
        }
    }

    fn render(&self, painter: &mut Painter, mouse_pos: Option<&Vec2<f32>>) {
        let container = &self.dims;

        let hovered = mouse_pos
            .map(|pos| container.contains(pos))
            .unwrap_or_default();

        let stroke_width = 20;

        let stroke_color = if hovered {
            Color::RED
        } else {
            Color::DARK_GRAY
        };

        painter.draw_primitive(
            quad()
                .rect(container.clone())
                .corners(Corners::with_all(10.0))
                .primitive()
                .fill_color(Color::WHITE)
                .stroke_color(stroke_color)
                .stroke_width(stroke_width),
        );

        painter.paint();
        // paint children clipped to this rect
        let mut clip = container.clone();
        let hsw = stroke_width.half() as f32;
        clip.origin.x += hsw;
        clip.origin.y += hsw;
        clip.size.width -= stroke_width as f32;
        clip.size.height -= stroke_width as f32;

        let mut cursor = container.origin + 10.0;
        let margin = 20.0;

        let size = Size {
            width: 100.0,
            height: 100.0,
        };

        let colors = [
            Color::BLACK,
            Color::KHAKI,
            Color::LIGHT_RED,
            Color::TORCH_RED,
            Color::DARK_BLUE,
        ];

        // paint children overflow hidden
        painter.paint_with_clip_rect(&clip, |painter| {
            for _ in 0..10 {
                for i in 0..10 {
                    painter.draw_primitive(
                        quad()
                            .rect(Rect::new_from_origin_size(
                                cursor + vec2(-self.scroll_x, 0.0),
                                size,
                            ))
                            .primitive()
                            .fill_color(colors[i % colors.len()]),
                    );
                    cursor.x += margin + size.width;
                }
                cursor.y += 30.0 + margin + size.height;
                cursor.x = container.origin.x + 10.0;
            }
        });
    }
}

async fn load_image_from_file_async<P: AsRef<Path>>(file_path: P) -> Result<RgbaImage> {
    let mut file = std::fs::File::open(file_path).map_err(|_| anyhow!("Error opening file"))?;

    let mut data = Vec::<u8>::new();
    file.read_to_end(&mut data)
        .map_err(|_| anyhow!("Error reading to end of file"))?;

    let loaded_image =
        image::load_from_memory(&data).map_err(|_| anyhow!("Error parsing image"))?;

    Ok(loaded_image.to_rgba8())
}
