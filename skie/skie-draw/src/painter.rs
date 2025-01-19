use std::sync::Arc;

use crate::{
    gpu::{
        error::GpuSurfaceCreateError,
        surface::{GpuSurface, GpuSurfaceSpecification},
    },
    paint::{AtlasKey, SkieAtlas, TextureKind},
    renderer::Renderable,
    Font, FontId, GlyphId, GlyphRenderSpecs, GpuContext, Primitive, Rect, Rgba, Scene, Size, Text,
    TextSystem, TextureId, Vec2, WgpuRenderer, WgpuRendererSpecs, Zero,
};

//  Winit window painter
#[derive(Debug)]
pub struct Painter {
    pub renderer: WgpuRenderer,
    pub(crate) scene: Scene,
    pub(crate) texture_system: Arc<SkieAtlas>,
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
        texture_system: Arc<SkieAtlas>,
        text_system: Arc<TextSystem>,
        specs: &WgpuRendererSpecs,
    ) -> Result<Self, GpuSurfaceCreateError> {
        let width = specs.width;
        let height = specs.height;

        let surface =
            gpu.create_surface(surface_target, &(GpuSurfaceSpecification { width, height }))?;
        let renderer = WgpuRenderer::new(gpu, &texture_system, specs);

        Ok(Self {
            renderer,
            surface,
            scene: Scene::default(),
            renderables: Default::default(),
            texture_system,
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

    pub fn draw_text(&mut self, _text: &Text) {
        // if text.text.is_empty() {
        //     return;
        // }
        //
        // let mut thing = text.text.chars();
        // let c = thing.next().unwrap();

        // self.texture_system.get_or_insert(
        //     &AtlasGlyph {
        //         font_id: FontId(0),
        //         glyph_id: GlyphId(0),
        //         font_size: text.size,
        //         scale_factor: 1.0,
        //     }
        //     .into(),
        //     || {
        //         let (rect, data) = self.text_system.rasterize_char(c, &text.font).unwrap();
        //         (TextureKind::Color, rect.size, &data)
        //     },
        // );
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
        let renderables =
            Self::build_renderables(&self.texture_system, scene, self.get_clip_rect());

        self.renderables.extend(renderables);
    }

    // commit renderables
    pub fn paint(&mut self) {
        let renderables =
            Self::build_renderables(&self.texture_system, &self.scene, self.get_clip_rect());

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
