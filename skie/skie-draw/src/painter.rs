use std::{borrow::Cow, sync::Arc};

use crate::{
    gpu::{
        error::GpuSurfaceCreateError,
        surface::{GpuSurface, GpuSurfaceSpecification},
    },
    paint::{AsPrimitive, AtlasKey, SkieAtlas, TextureKind},
    quad,
    renderer::Renderable,
    Color, GlyphImage, GpuContext, IsZero, Primitive, Rect, Rgba, Scene, Size, Text, TextSystem,
    TextureId, TextureOptions, Vec2, WgpuRenderer, WgpuRendererSpecs, Zero,
};
use anyhow::Result;
use cosmic_text::{Attrs, Buffer, Metrics, Shaping};
use wgpu::FilterMode;

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
        self.text_system.write(|state| {
            let metrics = Metrics::new(text.size, 1.4);
            let mut buffer = Buffer::new(&mut state.font_system, metrics);
            buffer.set_size(
                &mut state.font_system,
                Some(self.screen.width as f32),
                Some(self.screen.height as f32),
            );

            let attrs = Attrs::new();
            attrs.style(text.font.style.into());
            attrs.weight(text.font.weight.into());
            attrs.family(cosmic_text::Family::Name(&text.font.family));

            buffer.set_text(&mut state.font_system, &text.text, attrs, Shaping::Advanced);

            buffer.shape_until_scroll(&mut state.font_system, false);
            // begin run
            for run in buffer.layout_runs() {
                let line_y = run.line_y;

                // begin glyps
                for glyph in run.glyphs.iter() {
                    let scale = 1.0;
                    let physical_glyph = glyph.physical((text.pos.x, text.pos.y), scale);
                    let image = state
                        .swash_cache
                        .get_image(&mut state.font_system, physical_glyph.cache_key);

                    if let Some(image) = image {
                        let glyph_key = AtlasKey::from(GlyphImage(physical_glyph.cache_key));

                        let kind = match image.content {
                            cosmic_text::SwashContent::Color => TextureKind::Color,
                            cosmic_text::SwashContent::Mask => TextureKind::Mask,
                            // FIXME
                            cosmic_text::SwashContent::SubpixelMask => TextureKind::Mask,
                        };

                        let size =
                            Size::new(image.placement.width as i32, image.placement.height as i32);

                        if size.is_zero() {
                            continue;
                        };

                        self.texture_atlas
                            .get_or_insert(&glyph_key, || (kind, size, Cow::Borrowed(&image.data)));

                        self.renderer.set_texture_from_atlas(
                            &self.texture_atlas,
                            &glyph_key,
                            &TextureOptions::default()
                                .min_filter(FilterMode::Nearest)
                                .mag_filter(FilterMode::Nearest),
                        );

                        let x = physical_glyph.x + image.placement.left;
                        let y = line_y as i32 + physical_glyph.y - image.placement.top;
                        self.scene.add(
                            quad()
                                .rect(Rect::new_from_origin_size(
                                    (x as f32, y as f32).into(),
                                    size.map(|v| *v as f32),
                                ))
                                .primitive()
                                .textured(&TextureId::AtlasKey(glyph_key))
                                .fill_color(fill_color),
                        );
                    }
                }
                // end glyphs
            }
            // end run
        });

        self.paint();
    }

    pub fn paint_glyph() {}

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

    pub fn begin_frame(&mut self) {
        self.clear_all();
    }

    pub fn begin(&mut self) {
        unimplemented!()
    }

    /// Renders and presets to the screen
    pub fn finish(&mut self, clear_color: Rgba) {
        let Ok(cur_texture) = self.surface.surface.get_current_texture() else {
            // TODO: return error
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

            self.renderer.prepare(&self.renderables);
            self.renderer.render(&mut pass, &self.renderables);
        }

        self.renderer
            .gpu()
            .queue
            .submit(std::iter::once(encoder.finish()));

        cur_texture.present()
    }
}
