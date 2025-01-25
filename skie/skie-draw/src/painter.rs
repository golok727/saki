use std::{borrow::Cow, sync::Arc};

use crate::{
    paint::{AsPrimitive, AtlasKey, GpuTextureView, SkieAtlas, TextureKind},
    quad,
    renderer::Renderable,
    Circle, Color, GlyphImage, GpuContext, IsZero, Path2D, Primitive, Quad, Rect, Rgba, Scene,
    Size, Text, TextSystem, TextureId, TextureOptions, Vec2, WgpuRenderer, WgpuRendererSpecs, Zero,
};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping};
use wgpu::FilterMode;

#[derive(Default)]
pub struct PainterBuilder {
    pub texture_atlas: Option<Arc<SkieAtlas>>,
    pub text_system: Option<Arc<TextSystem>>,
    pub antialias: bool,
    pub size: Size<u32>,
}

impl PainterBuilder {
    pub fn new(size: Size<u32>) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    pub fn build(self, gpu: GpuContext) -> Painter {
        let texture_atlas = self
            .texture_atlas
            .unwrap_or(Arc::new(SkieAtlas::new(gpu.clone())));

        let text_system = self.text_system.unwrap_or(Arc::new(TextSystem::default()));

        let renderer = WgpuRenderer::new(
            gpu,
            &texture_atlas,
            &WgpuRendererSpecs {
                width: self.size.width,
                height: self.size.height,
            },
        );

        Painter {
            renderer,

            texture_atlas,
            text_system,

            screen: self.size,
            antialias: self.antialias,

            scene: Default::default(),
            renderables: Default::default(),
            clip_rects: Default::default(),
        }
    }

    pub fn with_size(mut self, size: Size<u32>) -> Self {
        self.size = size;
        self
    }

    pub fn with_texture_atlas(mut self, atlas: Arc<SkieAtlas>) -> Self {
        self.texture_atlas = Some(atlas);
        self
    }

    pub fn with_text_system(mut self, text_system: Arc<TextSystem>) -> Self {
        self.text_system = Some(text_system);
        self
    }

    pub fn antialias(mut self, val: bool) -> Self {
        self.antialias = val;
        self
    }
}

//  Winit window painter
pub struct Painter {
    pub renderer: WgpuRenderer,
    pub(crate) scene: Scene,
    pub(crate) texture_atlas: Arc<SkieAtlas>,
    pub(crate) text_system: Arc<TextSystem>,
    renderables: Vec<Renderable>,
    clip_rects: Vec<Rect<f32>>,
    screen: Size<u32>,
    antialias: bool,
    // todo msaa
}

impl Painter {
    pub fn create(size: Size<u32>) -> PainterBuilder {
        PainterBuilder::new(size)
    }

    pub fn atlas(&self) -> &Arc<SkieAtlas> {
        &self.texture_atlas
    }

    pub fn text_system(&self) -> &Arc<TextSystem> {
        &self.text_system
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

    pub fn clear(&mut self) {
        self.renderables.clear();
        self.scene.clear();
    }

    /// adds a primitive to th current scene does nothing until paint is called!
    pub fn paint_primitive(&mut self, prim: Primitive) {
        self.scene.add(prim)
    }

    pub fn paint_quad(&mut self, quad: Quad, style: impl FnOnce(Primitive) -> Primitive) {
        self.scene.add(style(quad.primitive()));
    }

    pub fn paint_circle(&mut self, circle: Circle, style: impl FnOnce(Primitive) -> Primitive) {
        self.scene.add(style(circle.primitive()));
    }

    pub fn paint_path(&mut self, path: Path2D, style: impl FnOnce(Primitive) -> Primitive) {
        self.scene.add(style(path.primitive()));
    }

    pub fn fill_text(&mut self, text: &Text, fill_color: Color) {
        self.text_system.write(|state| {
            let line_height_em = 1.4;
            let metrics = Metrics::new(text.size, text.size * line_height_em);
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
                        let kind = match image.content {
                            cosmic_text::SwashContent::Color => TextureKind::Color,
                            cosmic_text::SwashContent::Mask => TextureKind::Mask,
                            // we dont support it
                            cosmic_text::SwashContent::SubpixelMask => TextureKind::Mask,
                        };
                        let glyph_key = AtlasKey::from(GlyphImage {
                            key: physical_glyph.cache_key,
                            is_emoji: kind.is_color(),
                        });

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

        let tmp = self.antialias(false);
        self.paint();
        self.antialias(tmp);
    }
    pub fn antialias(&mut self, v: bool) -> bool {
        let old = self.antialias;
        self.antialias = v;
        old
    }

    pub fn paint_glyph() {}

    fn build_renderables<'scene>(
        texture_system: &SkieAtlas,
        scene: &'scene Scene,
        clip_rect: Rect<f32>,
        antialias: bool,
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

        scene
            .batches(info_map.clone(), antialias)
            .filter(|mesh| !mesh.is_empty())
            .map(move |mesh| Renderable {
                clip_rect: clip_rect.clone(),
                mesh,
            })
    }

    pub fn paint_scene(&mut self, scene: &Scene) {
        let renderables = Self::build_renderables(
            &self.texture_atlas,
            scene,
            self.get_clip_rect(),
            self.antialias,
        );

        self.renderables.extend(renderables);
    }

    pub fn with_clip_rect(&mut self, clip: &Rect<f32>, f: impl FnOnce(&mut Self)) {
        let cur_rect = self.get_clip_rect();
        self.clip_rects.push(cur_rect.intersect(clip));
        f(self);
        self.clip_rects.pop();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.screen = self.renderer.size();
    }

    // commit renderables
    // builds batched geometry for the current scene and clears the items
    pub fn paint(&mut self) {
        let renderables = Self::build_renderables(
            &self.texture_atlas,
            &self.scene,
            self.get_clip_rect(),
            self.antialias,
        );

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

    /// Renders and presets to the screen
    pub fn finish(&mut self, output_texture: &GpuTextureView, clear_color: Rgba) {
        let mut encoder = self.renderer.create_command_encoder();

        {
            let mut pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output_texture,
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
    }
}
