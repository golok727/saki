use crate::gpu::GpuContext;

pub trait RenderTarget: std::fmt::Debug + 'static {
    fn get_texture(&self) -> &wgpu::Texture;

    fn get_view(&self) -> wgpu::TextureView;

    fn resize(&mut self, width: u32, height: u32);

    fn update(&mut self, gpu: &GpuContext);

    fn prerender(&mut self) {}

    fn postrender(&mut self) {}
}

#[allow(unused)]
#[derive(Debug)]
pub struct RenderTargetImpl {
    texture: wgpu::Texture,
}

impl RenderTargetImpl {
    pub fn new() -> Self {
        todo!()
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        todo!("{new_width} {new_height}")
    }
}
