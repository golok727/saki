use std::sync::Arc;

use crate::{
    renderer::create_skie_renderer, GpuContext, Size, SkieAtlas, TextSystem, WgpuRendererSpecs,
};

use super::Canvas;

#[derive(Default)]
pub struct CanvasBuilder {
    pub texture_atlas: Option<Arc<SkieAtlas>>,
    pub text_system: Option<Arc<TextSystem>>,
    pub antialias: bool,
    pub size: Size<u32>,
}

impl CanvasBuilder {
    pub fn new(size: Size<u32>) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    pub fn build(self, gpu: GpuContext) -> Canvas {
        let texture_atlas = self
            .texture_atlas
            .unwrap_or(Arc::new(SkieAtlas::new(gpu.clone())));

        let text_system = self.text_system.unwrap_or(Arc::new(TextSystem::default()));

        let renderer = create_skie_renderer(
            gpu,
            &texture_atlas,
            &WgpuRendererSpecs {
                width: self.size.width,
                height: self.size.height,
            },
        );

        Canvas {
            renderer,

            texture_atlas,
            text_system,

            screen: self.size,
            antialias: self.antialias,

            scene: Default::default(),
            renderables: Default::default(),
            clip_rects: Default::default(),
        }
    }

    pub fn with_size(mut self, size: Size<u32>) -> Self {
        self.size = size;
        self
    }

    pub fn with_texture_atlas(mut self, atlas: Arc<SkieAtlas>) -> Self {
        self.texture_atlas = Some(atlas);
        self
    }

    pub fn with_text_system(mut self, text_system: Arc<TextSystem>) -> Self {
        self.text_system = Some(text_system);
        self
    }

    pub fn antialias(mut self, val: bool) -> Self {
        self.antialias = val;
        self
    }
}
