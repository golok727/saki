use crate::gpu::GpuContext;
use crate::math::{DevicePixels, Rect, Size};

use parking_lot::Mutex;
use std::sync::Arc;

use super::{TextureFormat, TextureKind};

#[derive(Debug, Default)]
struct AtlasTextureList<T: std::fmt::Debug> {
    slots: Vec<T>,
    free_slots: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct AtlasSystem(Arc<Mutex<AtlasSystemState>>);

unsafe impl Send for AtlasSystem {}

#[derive(Debug)]
struct AtlasSystemState {
    gpu: Arc<GpuContext>,
    gray_textures: AtlasTextureList<Option<AtlasTexture>>,
    color_textures: AtlasTextureList<Option<AtlasTexture>>,
}

impl AtlasSystem {
    pub fn new(gpu: Arc<GpuContext>) -> Self {
        Self(Arc::new(Mutex::new(AtlasSystemState {
            gpu,
            gray_textures: Default::default(),
            color_textures: Default::default(),
        })))
    }

    pub fn get() {}

    pub fn allocate_texture(&self, _size: Size<DevicePixels>, _kind: TextureKind) {
        let _lock = self.0.lock();
    }

    pub fn insert() {}
}

impl AtlasSystemState {
    pub fn new_texture(&mut self, size: Size<DevicePixels>, kind: TextureKind) -> AtlasTextureId {
        const DEFAULT_SIZE: Size<DevicePixels> = Size {
            width: DevicePixels(1024),
            height: DevicePixels(1024),
        };

        let size = DEFAULT_SIZE.max(&size);
        let format = kind.get_texture_format();

        let raw = self.gpu.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d {
                width: size.width.into(),
                height: size.height.into(),
                depth_or_array_layers: 1,
            },
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        });

        let bytes_per_pixel = kind.bytes_per_pixel();
        let width = raw.width();
        let height = raw.height();

        let n_bytes = width * height * bytes_per_pixel;

        let init_data = vec![0u8; n_bytes as usize];

        self.gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &raw,
                aspect: wgpu::TextureAspect::All,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &init_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width * bytes_per_pixel),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());

        let allocator = etagere::BucketedAtlasAllocator::new(size.into());

        let atlas_tex = AtlasTexture {
            allocator,
            kind,
            raw,
            view,
            format,
            size,
        };

        AtlasTextureId {
            kind,
            slot: todo!(),
        }
    }
}

struct AtlasTexture {
    allocator: etagere::BucketedAtlasAllocator,
    // FIXME Only TextureKind::Color is supported by renderer for now
    kind: TextureKind,
    raw: wgpu::Texture,
    view: wgpu::TextureView,
    format: TextureFormat,
    size: Size<DevicePixels>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AtlasTextureId {
    kind: TextureKind,
    slot: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AtlasTileId(u32);

#[derive(Debug, Clone, PartialEq)]
pub struct AtlasTile {
    id: AtlasTileId,
    texture: AtlasTextureId,
    bounds: Rect<DevicePixels>,
}

pub fn create_atlas_system(gpu: Arc<GpuContext>) -> Arc<AtlasSystem> {
    Arc::new(AtlasSystem::new(gpu))
}

impl std::fmt::Debug for AtlasTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Atlas")
            .field(
                "allocator",
                &format!("space = {}", self.allocator.allocated_space()),
            )
            .finish()
    }
}

impl From<etagere::Size> for Size<DevicePixels> {
    fn from(value: etagere::Size) -> Self {
        Self {
            width: value.width.into(),
            height: value.height.into(),
        }
    }
}

impl From<Size<DevicePixels>> for etagere::Size {
    fn from(value: Size<DevicePixels>) -> Self {
        etagere::size2(value.width.into(), value.height.into())
    }
}

impl From<etagere::AllocId> for AtlasTileId {
    fn from(value: etagere::AllocId) -> Self {
        Self(value.serialize())
    }
}

impl From<AtlasTileId> for etagere::AllocId {
    fn from(value: AtlasTileId) -> Self {
        etagere::AllocId::deserialize(value.0)
    }
}
