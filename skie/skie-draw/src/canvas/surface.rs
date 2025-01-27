use crate::GpuContext;
use anyhow::Result;
use wgpu::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use super::Canvas;

pub trait CanvasSurface {
    type PaintOutput;
    fn resize(&mut self, gpu: &GpuContext, new_width: u32, new_height: u32);
    fn paint(&mut self, canvas: &mut Canvas) -> Result<Self::PaintOutput>;
}

pub struct OffscreenRenderTarget {
    // pub for now
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
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

fn create_texture_fb_texture(gpu: &GpuContext, width: u32, height: u32) -> wgpu::Texture {
    gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some("framebuffer"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
