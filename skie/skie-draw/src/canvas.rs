use std::{borrow::Cow, sync::Arc};

use crate::{
    circle,
    paint::{
        AtlasKey, Brush, GpuTextureView, GraphicsInstruction, GraphicsTextureInfoMap,
        InstructionBatcher, Primitive, RenderList, SkieAtlas, TextureKind,
    },
    quad,
    renderer::Renderable,
    AtlasTextureInfo, BackendRenderTarget, Color, DrawList, GlyphImage, GpuSurfaceCreateError,
    GpuSurfaceSpecification, IsZero, Path2D, Rect, Size, Text, TextSystem, TextureId,
    TextureOptions, Vec2, WgpuRenderer,
};
use ahash::HashSet;
use anyhow::Result;
use cosmic_text::{Attrs, Buffer, Metrics, Shaping};
use skie_math::{vec2, Corners, Mat3, One, Zero};
use surface::{CanvasSurface, OffscreenRenderTarget};
use wgpu::FilterMode;

mod builder;
pub mod surface;

pub use builder::CanvasBuilder;

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    translation: Vec2<f32>,
    scale: Vec2<f32>,
    rotation: f32,
}

impl Transform {
    const NOOP: Self = Self {
        translation: Vec2 { x: 0.0, y: 0.0 },
        scale: Vec2 { x: 1.0, y: 1.0 },
        rotation: 0.0,
    };

    pub fn is_noop(&self) -> bool {
        self == &Self::NOOP
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec2::zero(),
            scale: Vec2::one(),
            rotation: 0.0,
        }
    }
}

impl From<&Transform> for Mat3 {
    fn from(transform: &Transform) -> Self {
        let mut mat = Mat3::identity();

        mat.scale(transform.scale.x, transform.scale.y);

        if transform.rotation != 0.0 {
            mat.rotate(transform.rotation);
        }

        mat.translate(transform.translation.x, transform.translation.y);

        mat
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasState {
    pub transform: Mat3,
    pub clip_rect: Rect<f32>,
}

#[derive(Debug)]
struct StagedInstructions {
    instructions: Vec<GraphicsInstruction>,
    state: CanvasState,
}

/*
 TODO
 - [ ] Shared path
*/
pub struct Canvas {
    // TODO pub(crate)
    pub renderer: WgpuRenderer,

    pub(crate) list: RenderList,
    pub(crate) texture_atlas: Arc<SkieAtlas>,
    pub(crate) text_system: Arc<TextSystem>,

    state_stack: Vec<CanvasState>,
    pub current_state: CanvasState,

    stage: Vec<StagedInstructions>,
    cached_renderables: Vec<Renderable>,

    clear_color: Color,
    screen: Size<u32>,
    // TODO msaa
}

impl Canvas {
    pub fn create(size: Size<u32>) -> CanvasBuilder {
        CanvasBuilder::new(size)
    }

    pub fn create_offscreen_target(&self) -> OffscreenRenderTarget {
        OffscreenRenderTarget::new(self.renderer.gpu(), self.width(), self.height())
    }

    pub fn create_backend_target<'window>(
        &self,
        into_surface_target: impl Into<wgpu::SurfaceTarget<'window>>,
    ) -> Result<BackendRenderTarget<'window>, GpuSurfaceCreateError> {
        self.renderer.gpu().create_surface(
            into_surface_target,
            &GpuSurfaceSpecification {
                width: self.width(),
                height: self.height(),
            },
        )
    }

    pub fn screen(&self) -> Size<u32> {
        self.screen
    }

    pub fn width(&self) -> u32 {
        self.screen.width
    }

    pub fn height(&self) -> u32 {
        self.screen.height
    }

    pub fn atlas(&self) -> &Arc<SkieAtlas> {
        &self.texture_atlas
    }

    pub fn text_system(&self) -> &Arc<TextSystem> {
        &self.text_system
    }

    pub fn get_clip_rect(&self) -> Rect<f32> {
        self.current_state.clip_rect.clone()
    }

    pub fn save(&mut self) {
        self.stage_changes();
        self.state_stack.push(self.current_state.clone());
    }

    pub fn clear_color(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
    }

    pub fn restore(&mut self) {
        if let Some(state) = self.state_stack.pop() {
            let restored = state;

            if restored != self.current_state {
                self.stage_changes();
            }

            self.current_state = restored;
        }
    }

    pub fn reset(&mut self) {
        self.stage_changes();

        self.clear_color = Color::WHITE;
        self.current_state = CanvasState {
            transform: Mat3::identity(),
            clip_rect: Rect::EVERYTHING,
        };

        self.state_stack.clear();
    }

    pub fn clip(&mut self, rect: &Rect<f32>) {
        self.stage_changes();
        self.current_state.clip_rect = self.current_state.clip_rect.intersect(rect);
    }

    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.stage_changes();
        self.current_state.transform.translate(dx, dy);
    }

    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.stage_changes();
        self.current_state.transform.scale(sx, sy);
    }

    pub fn rotate(&mut self, angle_rad: f32) {
        self.stage_changes();
        self.current_state.transform.rotate(angle_rad);
    }

    pub fn clear(&mut self) {
        self.stage.clear();
        self.list.clear();
        self.cached_renderables.clear();
    }

    pub fn stage_changes(&mut self) {
        if self.list.is_empty() {
            return;
        }

        let instructions = self.list.clear();

        self.stage.push(StagedInstructions {
            instructions,
            state: self.current_state.clone(),
        });
    }

    /// adds a primitive to th current scene does nothing until paint is called!
    #[inline]
    pub fn draw_primitive(&mut self, prim: impl Into<Primitive>, brush: &Brush) {
        self.list
            .add(GraphicsInstruction::brush(prim, brush.clone()));
    }

    pub fn draw_path(&mut self, path: Path2D, brush: &Brush) {
        self.draw_primitive(path, brush);
    }

    pub fn draw_rect(&mut self, rect: &Rect<f32>, brush: &Brush) {
        self.draw_primitive(quad().rect(rect.clone()), brush);
    }

    pub fn draw_round_rect(&mut self, rect: &Rect<f32>, corners: &Corners<f32>, brush: &Brush) {
        self.draw_primitive(quad().rect(rect.clone()).corners(corners.clone()), brush);
    }

    pub fn draw_image(&mut self, rect: &Rect<f32>, texture_id: &TextureId) {
        self.list.add(GraphicsInstruction::textured(
            quad().rect(rect.clone()),
            texture_id.clone(),
        ));
    }

    pub fn draw_image_rounded(
        &mut self,
        rect: &Rect<f32>,
        corners: &Corners<f32>,
        texture_id: &TextureId,
    ) {
        self.list.add(GraphicsInstruction::textured(
            quad().rect(rect.clone()).corners(corners.clone()),
            texture_id.clone(),
        ));
    }

    pub fn draw_circle(&mut self, cx: f32, cy: f32, radius: f32, brush: &Brush) {
        self.draw_primitive(circle().pos(cx, cy).radius(radius), brush);
    }

    pub fn fill_text(&mut self, text: &Text, fill_color: Color) {
        self.stage_changes();
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
                            // we dont support it for now
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
                            .get_or_insert(&glyph_key, || (size, Cow::Borrowed(&image.data)));

                        self.renderer.set_texture_from_atlas(
                            &self.texture_atlas,
                            &glyph_key,
                            &TextureOptions::default()
                                .min_filter(FilterMode::Nearest)
                                .mag_filter(FilterMode::Nearest),
                        );

                        let x = physical_glyph.x + image.placement.left;
                        let y = line_y as i32 + physical_glyph.y - image.placement.top;

                        let color = if kind.is_color() {
                            let mut c = Color::WHITE;
                            c.a = fill_color.a;
                            c
                        } else {
                            fill_color
                        };

                        self.list.add(GraphicsInstruction::textured_brush(
                            quad().rect(Rect::from_origin_size(
                                (x as f32, y as f32).into(),
                                size.map(|v| *v as f32),
                            )),
                            TextureId::AtlasKey(glyph_key),
                            Brush::filled(color),
                        ));
                    }
                }
                // end glyphs
            }
            // end run
        });
        self.stage_changes();
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.screen = self.renderer.size();
    }

    /// Resizes the surface and paints to it
    pub fn render<Output>(
        &mut self,
        surface: &mut impl CanvasSurface<PaintOutput = Output>,
    ) -> Result<Output> {
        surface.resize(self.renderer.gpu(), self.width(), self.height());
        surface.paint(self)
    }

    /// Renders and presets to the screen
    pub(crate) fn render_to_texture(&mut self, output_texture: &GpuTextureView) {
        self.prepare_for_render();

        let mut encoder = self.renderer.create_command_encoder();

        {
            let mut pass = encoder.begin_render_pass(
                &(wgpu::RenderPassDescriptor {
                    label: Some("RenderTarget Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: output_texture,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(self.clear_color.into()),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }),
            );

            self.renderer.prepare(&self.cached_renderables);
            self.renderer.render(&mut pass, &self.cached_renderables);
        }

        self.renderer
            .gpu()
            .queue
            .submit(std::iter::once(encoder.finish()));
    }

    fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.stage
            .iter()
            .flat_map(|staged| staged.instructions.iter())
            .map(|instruction| instruction.texture_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
    }

    fn prepare_for_render(&mut self) {
        // stage the any remaining ones
        self.stage_changes();

        let required_textures =
            self.get_required_textures()
                .filter_map(|tex| -> Option<AtlasKey> {
                    if let TextureId::AtlasKey(key) = tex {
                        Some(key)
                    } else {
                        None
                    }
                });

        let info_map = self.texture_atlas.get_texture_infos(required_textures);

        for staged in &self.stage {
            let batcher = InstructionBatcher::new(&staged.instructions, &info_map);
            for batch in batcher {
                let texture = batch.render_texture();

                if let Some(renderable) =
                    self.build_renderable(batch, &texture, &info_map, &staged.state)
                {
                    self.cached_renderables.push(renderable)
                }
            }
        }

        self.stage.clear();
    }

    fn build_renderable<'a>(
        &self,
        instructions: impl Iterator<Item = &'a GraphicsInstruction>,
        texture: &TextureId,
        tex_info: &GraphicsTextureInfoMap,
        canvas_state: &CanvasState,
    ) -> Option<Renderable> {
        let mut drawlist = DrawList::default();
        for instruction in instructions {
            let primitive = &instruction.primitive;
            let brush = &instruction.brush;

            if instruction.nothing_to_draw() {
                return None;
            }

            let tex_id = instruction.texture_id.clone();
            let is_white_texture = tex_id == TextureId::WHITE_TEXTURE;

            let info: Option<&AtlasTextureInfo> = if let TextureId::AtlasKey(key) = &tex_id {
                tex_info.get(key)
            } else {
                None
            };

            let build = |drawlist: &mut DrawList| match &primitive {
                Primitive::Circle(circle) => {
                    let fill_color = brush.fill_style.color;

                    drawlist.path.clear();
                    drawlist.path.circle(circle.center, circle.radius);

                    drawlist.fill_path_convex(fill_color, !is_white_texture);
                    drawlist.stroke_path(&brush.stroke_style.join())
                }

                Primitive::Quad(quad) => {
                    let fill_color = brush.fill_style.color;

                    drawlist.path.clear();
                    drawlist.path.round_rect(&quad.bounds, &quad.corners);
                    drawlist.fill_path_convex(fill_color, !is_white_texture);
                    drawlist.stroke_path(&brush.stroke_style.join())
                }

                Primitive::Path(path) => {
                    // TODO:
                    // drawlist.fill_with_path(path, prim.fill.color);

                    let stroke_style = if path.closed {
                        brush.stroke_style.join()
                    } else {
                        brush.stroke_style
                    };

                    drawlist.stroke_with_path(path, &stroke_style);
                }
            };

            let identity_transform = canvas_state.transform.is_identity();

            if identity_transform && info.is_none() {
                build(&mut drawlist)
            } else {
                drawlist.capture(build).map(|vertex| {
                    if let Some(info) = info {
                        if is_white_texture {
                            vertex.uv = info.uv_to_atlas_space(0.0, 0.0).into();
                        } else {
                            vertex.uv = info.uv_to_atlas_space(vertex.uv[0], vertex.uv[1]).into();
                        }
                    }

                    if !identity_transform {
                        let pos =
                            canvas_state.transform * vec2(vertex.position[0], vertex.position[1]);
                        vertex.position = [pos.x, pos.y];
                    }
                });
            }
        }

        let mut mesh = drawlist.build();
        if mesh.is_empty() {
            return None;
        }

        mesh.texture = texture.clone();

        Some(Renderable {
            clip_rect: canvas_state.clip_rect.clone(),
            mesh,
        })
    }
}
