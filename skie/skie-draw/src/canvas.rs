use std::{borrow::Cow, sync::Arc};

use crate::{
    circle,
    paint::{AsPrimitive, AtlasKey, Brush, GpuTextureView, SkieAtlas, TextureKind},
    quad,
    renderer::Renderable,
    Color, GlyphImage, IsZero, Path2D, Primitive, Rect, Rgba, Scene, Size, Text, TextSystem,
    TextureId, TextureOptions, Vec2, WgpuRenderer, Zero,
};
use cosmic_text::{Attrs, Buffer, Metrics, Shaping};
use skie_math::{Corners, Mat3};
use wgpu::FilterMode;

mod builder;
pub use builder::CanvasBuilder;

#[derive(Debug, Clone)]
pub(crate) struct CanvasState {
    pub transform: Mat3,
    pub clip: Rect<f32>,
}

/*
 TODO
 - [ ] Shared path
 - [ ] transforms and cliprect saves instead of using with_clip_rect
 - [ ] use new brush api to paint
*/
pub struct Canvas {
    pub renderer: WgpuRenderer,
    pub(crate) scene: Scene,
    pub(crate) texture_atlas: Arc<SkieAtlas>,
    pub(crate) text_system: Arc<TextSystem>,
    state_stack: Vec<CanvasState>,
    current_state: CanvasState,
    renderables: Vec<Renderable>,
    clip_rects: Vec<Rect<f32>>,
    screen: Size<u32>,
    antialias: bool,
    // TODO msaa
}

impl Canvas {
    pub fn screen(&self) -> Size<u32> {
        self.screen
    }

    pub fn width(&self) -> u32 {
        self.screen.width
    }

    pub fn height(&self) -> u32 {
        self.screen.height
    }

    pub fn create(size: Size<u32>) -> CanvasBuilder {
        CanvasBuilder::new(size)
    }

    pub fn atlas(&self) -> &Arc<SkieAtlas> {
        &self.texture_atlas
    }

    pub fn text_system(&self) -> &Arc<TextSystem> {
        &self.text_system
    }

    pub fn save(&mut self) {
        self.state_stack.push(self.current_state.clone());
    }

    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            self.current_state = state;
        }
    }

    #[cfg(feature = "experimental")]
    pub fn clip(&mut self, _rect: &Rect<f32>) {}

    #[cfg(feature = "experimental")]
    pub fn translate(&mut self, x: f32, y: f32) {
        dbg!(x, y);
    }

    #[cfg(feature = "experimental")]
    pub fn scale(&mut self, x: f32, y: f32) {
        dbg!(x, y);
    }

    #[cfg(feature = "experimental")]
    pub fn rotate(&mut self, x: f32, y: f32) {
        dbg!(x, y);
    }

    pub fn get_clip_rect(&self) -> Rect<f32> {
        self.clip_rects
            .last()
            .cloned()
            .unwrap_or(Rect::from_origin_size(
                Vec2::zero(),
                self.screen.map_cloned(|v| v as f32),
            ))
    }

    pub fn clear(&mut self) {
        self.renderables.clear();
        self.scene.clear();
    }

    /// adds a primitive to th current scene does nothing until paint is called!
    pub fn draw_primitive(&mut self, prim: impl Into<Primitive>, brush: &Brush) {
        self.scene.add(
            prim.primitive()
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
    }

    pub fn draw_path(&mut self, path: Path2D, brush: &Brush) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            path.primitive()
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
    }

    pub fn draw_rect(&mut self, rect: &Rect<f32>, brush: &Brush) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            quad()
                .rect(rect.clone())
                .primitive()
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
    }

    pub fn draw_round_rect(&mut self, rect: &Rect<f32>, corners: &Corners<f32>, brush: &Brush) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            quad()
                .rect(rect.clone())
                .corners(corners.clone())
                .primitive()
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
    }

    pub fn draw_image(&mut self, rect: &Rect<f32>, texture_id: &TextureId) {
        // for NOW Only for playing around with the new api
        self.scene
            .add(quad().rect(rect.clone()).primitive().textured(texture_id))
    }

    pub fn draw_image_rounded(
        &mut self,
        rect: &Rect<f32>,
        corners: &Corners<f32>,
        texture_id: &TextureId,
    ) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            quad()
                .rect(rect.clone())
                .corners(corners.clone())
                .primitive()
                .textured(texture_id),
        )
    }

    pub fn draw_image_2(&mut self, rect: &Rect<f32>, texture_id: &TextureId, brush: &Brush) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            quad()
                .rect(rect.clone())
                .primitive()
                .textured(texture_id)
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
    }

    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, brush: &Brush) {
        // for NOW Only for playing around with the new api
        self.scene.add(
            circle()
                .pos(cx, cy)
                .radius(radius)
                .primitive()
                .fill(brush.fill_style)
                .stroke(brush.stroke_style),
        )
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
                                .rect(Rect::from_origin_size(
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

    // TODO: automaticaly do it instead ( will be removed )
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
