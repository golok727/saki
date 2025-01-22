use std::{borrow::Cow, sync::Arc};

use crate::{
    arc_string::ArcString,
    gpu::{
        error::GpuSurfaceCreateError,
        surface::{GpuSurface, GpuSurfaceSpecification},
    },
    paint::{AsPrimitive, AtlasKey, SkieAtlas, TextureKind},
    quad,
    renderer::Renderable,
    Color, Font, GlyphRenderSpecs, GpuContext, Primitive, Rect, Rgba, Scene, Size, Text,
    TextSystem, TextureFilterMode, TextureId, TextureOptions, Vec2, WgpuRenderer,
    WgpuRendererSpecs, Zero,
};
use anyhow::Result;

//  Winit window painter
pub struct Painter {
    pub renderer: WgpuRenderer,
    pub(crate) scene: Scene,
    pub(crate) texture_atlas: Arc<SkieAtlas>,
    pub(crate) text_system: Arc<TextSystem>,
    pub(crate) surface: GpuSurface,
    renderables: Vec<Renderable>,
    clip_rects: Vec<Rect<f32>>,
    screen: Size<u32>,
    // todo msaa
}

impl Painter {
    pub fn new(
        gpu: Arc<GpuContext>,
        surface_target: impl Into<wgpu::SurfaceTarget<'static>>,
        texture_atlas: Arc<SkieAtlas>,
        text_system: Arc<TextSystem>,
        specs: &WgpuRendererSpecs,
    ) -> Result<Self, GpuSurfaceCreateError> {
        let width = specs.width;
        let height = specs.height;

        let surface =
            gpu.create_surface(surface_target, &(GpuSurfaceSpecification { width, height }))?;
        let renderer = WgpuRenderer::new(gpu, &texture_atlas, specs);

        Ok(Self {
            renderer,
            surface,
            scene: Scene::default(),
            renderables: Default::default(),
            texture_atlas,
            text_system,
            screen: Size {
                width: specs.width,
                height: specs.height,
            },
            clip_rects: Default::default(),
        })
    }

    pub fn get_clip_rect(&self) -> Rect<f32> {
        self.clip_rects
            .last()
            .cloned()
            .unwrap_or(Rect::new_from_origin_size(
                Vec2::zero(),
                self.screen.map_cloned(|v| v as f32),
            ))
    }

    pub fn begin_frame(&mut self) {
        self.clear_all();
    }

    pub fn clear_all(&mut self) {
        self.renderables.clear();
        self.scene.clear();
    }

    pub fn clear_staged(&mut self) {
        self.renderables.clear();
    }

    pub fn clear_unstaged(&mut self) {
        self.scene.clear();
    }

    /// adds a primitive to th current scene does nothing until paint is called!
    pub fn draw_primitive(&mut self, prim: Primitive) {
        self.scene.add(prim)
    }

    pub fn draw_text(&mut self, text: &Text, fill_color: Color) {
        let font = Font::new(ArcString::new_static("Segoe UI")).bold();
        let font_id = self.text_system.font_id(&font).unwrap();

        let mut cursor_x = 0.0;

        for c in text.text.chars() {
            let glyph_id = self.text_system.glyph_for_char(font_id, c).unwrap();

            let glyph_specs = GlyphRenderSpecs {
                font_id,
                glyph_id,
                font_size: text.size,
                scale_factor: 1.0,
            };

            let key = AtlasKey::Glyf(glyph_specs);
            let (raster_bounds, data) = self.text_system.rasterize(&glyph_specs).unwrap();
            let tile = self.texture_atlas.get_or_insert(&key, || {
                dbg!(c, &raster_bounds);
                (TextureKind::Mask, raster_bounds.size, Cow::Owned(data))
            });

            self.renderer.set_texture_from_atlas(
                &self.texture_atlas,
                &key,
                &TextureOptions::default()
                    .min_filter(TextureFilterMode::Nearest)
                    .mag_filter(TextureFilterMode::Nearest),
            );

            let size = tile.bounds.size.map(|c| *c as f32);

            let pos = Vec2 {
                x: text.pos.x.floor() + cursor_x,
                y: text.pos.y.floor(),
            } + raster_bounds.origin.map(|v| *v as f32);

            self.scene.add(
                quad()
                    .rect(Rect::new_from_origin_size(pos, size))
                    .primitive()
                    .textured(&key.into())
                    .fill_color(fill_color)
                    .stroke_width(2)
                    .stroke_color(Color::RED),
            );

            // debug
            self.scene.add(
                quad()
                    .rect(Rect::new_from_origin_size(pos, size))
                    .primitive()
                    .stroke_width(2)
                    .stroke_color(Color::RED),
            );
            cursor_x += size.width;
        }

        self.paint();
    }

    fn build_renderables<'scene>(
        texture_system: &SkieAtlas,
        scene: &'scene Scene,
        clip_rect: Rect<f32>,
    ) -> impl Iterator<Item = Renderable> + 'scene {
        let atlas_textures = scene
            .get_required_textures()
            .filter_map(|tex| -> Option<AtlasKey> {
                if let TextureId::AtlasKey(key) = tex {
                    Some(key)
                } else {
                    None
                }
            });

        let info_map = texture_system.get_texture_infos(atlas_textures);

        scene.batches(info_map.clone()).map(move |mesh| Renderable {
            clip_rect: clip_rect.clone(),
            mesh,
        })
    }

    pub fn paint_scene(&mut self, scene: &Scene) {
        let renderables = Self::build_renderables(&self.texture_atlas, scene, self.get_clip_rect());

        self.renderables.extend(renderables);
    }

    // commit renderables
    pub fn paint(&mut self) {
        let renderables =
            Self::build_renderables(&self.texture_atlas, &self.scene, self.get_clip_rect());

        self.renderables.extend(renderables);
        self.scene.clear();
    }

    pub fn paint_with_clip_rect(&mut self, clip: &Rect<f32>, f: impl FnOnce(&mut Self)) {
        let cur_rect = self.get_clip_rect();

        self.clip_rects.push(cur_rect.intersect(clip));
        f(self);
        self.paint();
        self.clip_rects.pop();
    }

    pub fn with_clip_rect(&mut self, clip: &Rect<f32>, f: impl FnOnce(&mut Self)) {
        let cur_rect = self.get_clip_rect();
        self.clip_rects.push(cur_rect.intersect(clip));
        f(self);
        self.clip_rects.pop();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.surface.resize(self.renderer.gpu(), width, height);
        self.screen = *self.renderer.size();
    }

    /// Renders and presets to the screen
    pub fn finish(&mut self, clear_color: Rgba) {
        let Ok(cur_texture) = self.surface.surface.get_current_texture() else {
            // TODO: return error ?
            log::error!("Error getting texture");
            return;
        };

        let view = cur_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.renderer.create_command_encoder();

        {
            let mut pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color.into()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }),
            );

            self.renderer.set_renderables(&self.renderables);
            self.renderer.render(&mut pass, &self.renderables);
        }

        self.renderer
            .gpu()
            .queue
            .submit(std::iter::once(encoder.finish()));

        cur_texture.present()
    }
}
