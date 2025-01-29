use crate::GpuContext;

use super::{snapshot::CanvasSnapshotSource, surface::CanvasSurface, Canvas};
use anyhow::Result;

pub struct OffscreenRenderTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl OffscreenRenderTarget {
    // todo allow config
    pub(super) fn new(gpu: &GpuContext, width: u32, height: u32) -> Self {
        let texture = create_texture_fb_texture(gpu, width, height);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { texture, view }
    }
}

impl CanvasSurface for OffscreenRenderTarget {
    type PaintOutput = ();

    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput> {
        canvas.render_to_texture(&self.view);
        Ok(())
    }

    fn resize(&mut self, gpu: &GpuContext, new_width: u32, new_height: u32) {
        if self.texture.width() == new_width && self.texture.height() == new_height {
            return;
        }

        let width = new_width.max(1);
        let height = new_height.max(1);

        let texture = create_texture_fb_texture(gpu, width, height);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.texture = texture;
        self.view = view;
    }
}

impl CanvasSnapshotSource for OffscreenRenderTarget {
    fn get_output_texture(&self) -> wgpu::Texture {
        self.texture.clone()
    }
}

fn create_texture_fb_texture(gpu: &GpuContext, width: u32, height: u32) -> wgpu::Texture {
    gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("framebuffer"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
