use std::collections::VecDeque;

use crate::{
    paint::{AtlasKey, AtlasTextureInfoMap, Primitive},
    Brush, TextureId,
};

use ahash::HashSet;

use super::Color;

#[derive(Debug, Clone)]
pub struct GraphicsInstruction {
    pub primitive: Primitive,
    pub brush: Brush,
    pub texture_id: TextureId,
}

impl GraphicsInstruction {
    pub fn nothing_to_draw(&self) -> bool {
        self.brush.noting_to_draw()
    }

    pub fn textured(primitive: impl Into<Primitive>, texture_id: TextureId) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id,
            brush: Brush::filled(Color::WHITE),
        }
    }

    pub fn brush(primitive: impl Into<Primitive>, brush: Brush) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id: TextureId::WHITE_TEXTURE,
            brush,
        }
    }

    pub fn textured_brush(
        primitive: impl Into<Primitive>,
        texture_id: TextureId,
        brush: Brush,
    ) -> Self {
        Self {
            primitive: primitive.into(),
            texture_id,
            brush,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct RenderList {
    pub(crate) instructions: Vec<GraphicsInstruction>,
}

impl RenderList {
    pub fn add(&mut self, instruction: GraphicsInstruction) {
        self.instructions.push(instruction)
    }

    pub fn clear(&mut self) -> Vec<GraphicsInstruction> {
        let old: Vec<GraphicsInstruction> = std::mem::take(&mut self.instructions);
        old
    }

    pub fn get_required_textures(&self) -> impl Iterator<Item = TextureId> + '_ {
        self.instructions
            .iter()
            .map(|instruction| instruction.texture_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

type GroupEntry = usize;
struct Group {
    render_texture: TextureId,
    entries: Vec<GroupEntry>,
}

pub type GraphicsTextureInfoMap = AtlasTextureInfoMap<AtlasKey>;

// A simple batcher for now in future we will expand this.
pub(crate) struct InstructionBatcher<'a> {
    instructions: &'a [GraphicsInstruction],
    // (Actual RenderTexture bind to renderer, GroupEntry)
    groups: VecDeque<Group>,
}

impl<'a> InstructionBatcher<'a> {
    pub fn new(
        instructions: &'a [GraphicsInstruction],
        tex_info: &'a GraphicsTextureInfoMap,
    ) -> Self {
        let mut render_tex_to_item_idx: ahash::AHashMap<TextureId, Vec<GroupEntry>> =
            Default::default();

        for (i, instruction) in instructions.iter().enumerate() {
            let render_texture = match &instruction.texture_id {
                TextureId::AtlasKey(key) => {
                    let info = tex_info.get(key);
                    info.map(|info| TextureId::Atlas(info.tile.texture))
                }
                other => Some(other.clone()),
            };

            if let Some(render_texture) = render_texture {
                render_tex_to_item_idx
                    .entry(render_texture)
                    .or_default()
                    .push(i);
            }
        }

        let mut groups: Vec<_> = render_tex_to_item_idx
            .into_iter()
            .map(|(render_texture, entries)| Group {
                render_texture,
                entries,
            })
            .collect();

        // FIXME
        groups.sort_by_key(|group| group.entries.first().copied().unwrap_or(0));

        let groups: VecDeque<_> = groups.into();

        Self {
            instructions,
            groups,
        }
    }
}

impl<'a> Iterator for InstructionBatcher<'a> {
    type Item = InstructionBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(group) = self.groups.pop_front() {
            Some(InstructionBatch::<'a> {
                instructions: self.instructions,
                group,
                idx: 0,
            })
        } else {
            None
        }
    }
}

pub struct InstructionBatch<'a> {
    instructions: &'a [GraphicsInstruction],
    group: Group,
    idx: usize,
}

impl<'a> InstructionBatch<'a> {
    pub fn render_texture(&self) -> TextureId {
        self.group.render_texture.clone()
    }
}

impl<'a> Iterator for InstructionBatch<'a> {
    type Item = &'a GraphicsInstruction;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.group.entries.len() {
            return None;
        }

        let entry = &self.instructions[self.group.entries[self.idx]];
        self.idx += 1;
        Some(entry)
    }
}
