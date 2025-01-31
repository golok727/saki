use crate::GpuContext;

use super::{
    snapshot::CanvasSnapshotSource,
    surface::{CanvasSurface, CanvasSurfaceConfig},
    Canvas,
};
use anyhow::Result;

pub struct OffscreenRenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl OffscreenRenderTarget {
    pub(super) fn new(gpu: &GpuContext, config: &CanvasSurfaceConfig) -> Self {
        let texture = create_fb_texture(gpu, config);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }
}

impl CanvasSurface for OffscreenRenderTarget {
    type PaintOutput = ();
    const LABEL: &'static str = "OffscreenRenderTarget";

    fn configure(&mut self, gpu: &GpuContext, config: &CanvasSurfaceConfig) {
        debug_assert!(config.width != 0, "Got zero width");
        debug_assert!(config.height != 0, "Got zero heihgt");

        let texture = create_fb_texture(gpu, config);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture = texture;
        self.view = view;
    }

    fn get_config(&self) -> CanvasSurfaceConfig {
        CanvasSurfaceConfig {
            width: self.texture.width(),
            height: self.texture.height(),
            format: self.texture.format(),
            usage: self.texture.usage(),
        }
    }

    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput> {
        canvas.render_to_texture(&self.view);
        Ok(())
    }
}

impl Canvas {
    pub fn create_offscreen_target(&self) -> OffscreenRenderTarget {
        OffscreenRenderTarget::new(self.renderer.gpu(), &self.surface_config)
    }
}

impl CanvasSnapshotSource for OffscreenRenderTarget {
    fn get_source_texture(&self) -> wgpu::Texture {
        self.texture.clone()
    }
}

fn create_fb_texture(gpu: &GpuContext, config: &CanvasSurfaceConfig) -> wgpu::Texture {
    gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("framebuffer"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | config.usage,
        view_formats: &[],
    })
}
