use std::sync::Arc;

use skie_draw::{
    gpu::{
        error::GpuSurfaceCreateError,
        surface::{GpuSurface, GpuSurfaceSpecification},
        GpuContext,
    },
    math::{Rect, Size},
    paint::{atlas::AtlasManager, Rgba},
    renderer::Renderable,
    Scene, WgpuRenderer, WgpuRendererSpecs,
};
//  Winit window painter
#[derive(Debug)]
pub struct Painter {
    pub(crate) renderer: WgpuRenderer,
    pub(crate) surface: GpuSurface,
    pub(crate) scene: Scene,
    // todo mmsa
}

impl Painter {
    pub fn new(
        gpu: Arc<GpuContext>,
        texture_system: AtlasManager,
        window: Arc<winit::window::Window>,
        specs: &WgpuRendererSpecs,
    ) -> Result<Self, GpuSurfaceCreateError> {
        let width = specs.width;
        let height = specs.height;

        let surface = gpu.create_surface(window, &(GpuSurfaceSpecification { width, height }))?;
        let renderer = WgpuRenderer::new(gpu, texture_system, specs);

        Ok(Self {
            renderer,
            surface,
            scene: Scene::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        self.surface.resize(self.renderer.gpu(), width, height);
    }

    pub fn paint(&mut self, clear_color: Rgba, screen_size: Size<u32>) {
        let info_map = self
            .renderer
            .texture_system()
            .get_texture_infos(self.scene.get_required_textures());

        let scissor: Rect<u32> = Rect::new(0, 0, screen_size.width, screen_size.height);

        let renderables = self
            .scene
            .batches(info_map.clone())
            .map(|mesh| Renderable {
                clip_rect: scissor.clone(),
                mesh,
            })
            .collect::<Vec<_>>();

        let cur_texture = self.surface.surface.get_current_texture().unwrap();
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

            self.renderer.set_renderables(&renderables);
            self.renderer.render(&mut pass, &renderables);
        }

        self.renderer
            .gpu()
            .queue
            .submit(std::iter::once(encoder.finish()));

        cur_texture.present()
    }
}
