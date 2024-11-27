use crate::gpu::GpuContext;

pub type TextureFormat = wgpu::TextureFormat;

#[derive(Debug)]
pub struct RenderTargetSpecification {
    width: u32,
    height: u32,
    label: Option<&'static str>,
    format: wgpu::TextureFormat,
}

impl Default for RenderTargetSpecification {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            label: None,
            format: TextureFormat::Rgba8Unorm,
        }
    }
}

impl RenderTargetSpecification {
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct RenderTarget {
    width: u32,
    height: u32,
    texture_view: wgpu::TextureView,
    texture: wgpu::Texture,
    dirty: bool,
}

impl RenderTarget {
    pub fn new(gpu: &GpuContext, specs: &RenderTargetSpecification) -> Self {
        let texture = RenderTarget::create_render_target_texture(gpu, specs);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture_view,
            texture,
            width: specs.width,
            height: specs.height,
            dirty: false,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    fn create_render_target_texture(
        gpu: &GpuContext,
        specs: &RenderTargetSpecification,
    ) -> wgpu::Texture {
        gpu.create_texture(&wgpu::TextureDescriptor {
            label: specs.label,
            size: wgpu::Extent3d {
                width: specs.width,
                height: specs.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: specs.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if self.width == new_width && self.height == new_height {
            return;
        }

        let new_width = new_width.max(1);
        let new_height = new_height.max(1);

        self.width = new_width;
        self.height = new_height;

        self.dirty = true;
    }

    pub fn sync(&mut self, gpu: &GpuContext) {
        if !self.dirty {
            return;
        }
        self.dirty = false;

        let spec = RenderTargetSpecification::default()
            .with_size(self.width, self.height)
            .with_format(self.texture.format());

        self.texture = Self::create_render_target_texture(gpu, &spec);
        self.texture_view = self
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        log::trace!(
            "Render target resize: width = {} height = {}",
            self.width,
            self.height
        );
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn texture_view(&self) -> &wgpu::TextureView {
        &self.texture_view
    }

    pub fn copy_to_texture(&self, encoder: &mut wgpu::CommandEncoder, destination: &wgpu::Texture) {
        let src = wgpu::ImageCopyTexture {
            texture: &self.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::default(),
        };

        let dest = wgpu::ImageCopyTexture {
            texture: destination,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::default(),
        };

        encoder.copy_texture_to_texture(
            src,
            dest,
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }
}
