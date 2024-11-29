use crate::gpu::GpuContext;
use crate::math::{DevicePixels, Size};

use parking_lot::Mutex;
use std::sync::Arc;

use super::{TextureFormat, TextureKind};

pub trait AtlasSystem: std::fmt::Debug {
    fn get_or_insert(&mut self);
    fn get(&self);
    fn remove(&mut self);
}

pub type SharedAtlasSystem = Arc<dyn AtlasSystem>;

#[derive(Debug)]
pub struct SkiAtlasSystem(Mutex<AtlasSystemState>);

#[derive(Debug)]
struct AtlasSystemState {
    gpu: Arc<GpuContext>,
}

impl SkiAtlasSystem {
    pub fn new(gpu: Arc<GpuContext>) -> Self {
        Self(Mutex::new(AtlasSystemState { gpu }))
    }

    pub fn push_texture(&mut self) {}

    pub fn get(&self) {}
}

struct AtlasTexture {
    allocator: etagere::BucketedAtlasAllocator,
    // Only Color is supported for now
    kind: TextureKind,
    format: TextureFormat,
    size: Size<DevicePixels>,
}

#[derive(Debug)]
struct AtlasTile {}

impl AtlasSystem for SkiAtlasSystem {
    fn get_or_insert(&mut self) {
        todo!()
    }

    fn get(&self) {
        todo!()
    }

    fn remove(&mut self) {
        todo!()
    }
}

pub fn default_atlas_system(gpu: Arc<GpuContext>) -> SharedAtlasSystem {
    Arc::new(SkiAtlasSystem::new(gpu))
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
