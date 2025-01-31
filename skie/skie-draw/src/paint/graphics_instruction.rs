use std::collections::VecDeque;

use crate::{paint::Primitive, Brush, TextureId};

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

type GroupEntry = usize;
struct Group {
    render_texture: TextureId,
    entries: Vec<GroupEntry>,
}

// A simple batcher for now in future we will expand this.
pub(crate) struct GraphicsInstructionBatcher<'a> {
    instructions: &'a [GraphicsInstruction],
    groups: VecDeque<Group>,
}

impl<'a> GraphicsInstructionBatcher<'a> {
    pub fn new(
        instructions: &'a [GraphicsInstruction],
        get_renderer_texture_id: impl Fn(&TextureId) -> Option<TextureId>,
    ) -> Self {
        let mut render_tex_to_item_idx: ahash::AHashMap<TextureId, Vec<GroupEntry>> =
            Default::default();

        for (i, instruction) in instructions.iter().enumerate() {
            let renderer_texture = get_renderer_texture_id(&instruction.texture_id);

            if let Some(render_texture) = renderer_texture {
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

impl<'a> Iterator for GraphicsInstructionBatcher<'a> {
    type Item = InstructionBatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.groups.pop_front().map(|group| InstructionBatch::<'a> {
            instructions: self.instructions,
            group,
            idx: 0,
        })
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
