use crate::{math::Vec2, Pixels};

use skie_math::Size;
pub use taffy::style::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Display, FlexDirection, FlexWrap,
    JustifyContent, Overflow, Position,
};

use crate::{Edges, FixedLength, Length};

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub display: Display,
    pub margin: Edges<Length>,
    pub padding: Edges<FixedLength>,
    pub border: Edges<FixedLength>,
    pub box_sizing: BoxSizing,
    pub position: Position,
    pub overflow: Vec2<Overflow>,
    pub size: Size<Length>,
    pub min_size: Size<Length>,
    pub max_size: Size<Length>,

    // align
    pub align_items: Option<AlignItems>,
    pub align_self: Option<AlignSelf>,
    pub justify_items: Option<AlignItems>,
    pub justify_self: Option<AlignSelf>,
    pub align_content: Option<AlignContent>,
    pub justify_content: Option<JustifyContent>,
    pub gap: Size<FixedLength>,

    // flex_things
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub flex_basis: Length,
    pub flex_grow: Pixels,
    pub flex_shrink: Pixels,
}
