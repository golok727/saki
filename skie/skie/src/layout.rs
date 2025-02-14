use skie_math::{Size, Vec2};
use taffy::{prelude::TaffyAuto, TaffyTree};

use crate::{style::Style, Edges, FixedLength, Length, Pixels};

pub struct NodeContent;

pub struct LayoutEngine {
    tree: taffy::TaffyTree<NodeContent>,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::<NodeContent>::new(),
        }
    }

    pub fn clear(&mut self) {
        self.tree.clear()
    }

    pub fn layout(&mut self, style: Style, rem_size: Pixels) {
        let _taffy = style.to_taffy(rem_size);
    }
}

impl ToTaffy<taffy::Style> for Style {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::Style {
        taffy::Style {
            display: self.display,
            box_sizing: self.box_sizing,
            position: self.position,
            overflow: self.overflow.to_taffy(rem_size),
            size: self.size.to_taffy(rem_size),
            min_size: self.min_size.to_taffy(rem_size),
            max_size: self.max_size.to_taffy(rem_size),
            margin: self.margin.to_taffy(rem_size),
            padding: self.padding.to_taffy(rem_size),
            border: self.border.to_taffy(rem_size),

            align_items: self.align_items,
            align_self: self.align_self,
            justify_items: self.justify_items,
            justify_self: self.justify_self,
            align_content: self.align_content,
            justify_content: self.justify_content,
            gap: self.gap.to_taffy(rem_size),

            flex_direction: self.flex_direction,
            flex_wrap: self.flex_wrap,
            flex_basis: self.flex_basis.to_taffy(rem_size),
            flex_grow: self.flex_grow.0,
            flex_shrink: self.flex_shrink.0,
            ..Default::default()
        }
    }
}

trait ToTaffy<Output> {
    fn to_taffy(&self, rem_size: Pixels) -> Output;
}

impl<T: Clone> ToTaffy<taffy::Point<T>> for Vec2<T> {
    fn to_taffy(&self, _rem_size: Pixels) -> taffy::Point<T> {
        taffy::Point {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }
}

impl<T: ToTaffy<INNER>, INNER> ToTaffy<taffy::Rect<INNER>> for Edges<T> {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::Rect<INNER> {
        taffy::Rect {
            top: self.top.to_taffy(rem_size),
            right: self.right.to_taffy(rem_size),
            bottom: self.bottom.to_taffy(rem_size),
            left: self.left.to_taffy(rem_size),
        }
    }
}

impl<T: ToTaffy<INNER>, INNER> ToTaffy<taffy::Size<INNER>> for Size<T> {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::Size<INNER> {
        taffy::Size {
            width: self.width.to_taffy(rem_size),
            height: self.height.to_taffy(rem_size),
        }
    }
}

impl ToTaffy<taffy::LengthPercentageAuto> for Length {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::LengthPercentageAuto {
        match self {
            Length::Auto => taffy::LengthPercentageAuto::AUTO,
            Length::Fixed(len) => len.to_taffy(rem_size),
        }
    }
}

impl ToTaffy<taffy::Dimension> for Length {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::Dimension {
        match self {
            Length::Auto => taffy::Dimension::AUTO,
            Length::Fixed(len) => len.to_taffy(rem_size),
        }
    }
}

impl ToTaffy<taffy::LengthPercentageAuto> for FixedLength {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::LengthPercentageAuto {
        match self {
            FixedLength::Absolute(length) => {
                taffy::LengthPercentageAuto::Length(length.to_pixels(rem_size).0)
            }
            FixedLength::Percent(percent) => taffy::LengthPercentageAuto::Percent(*percent),
        }
    }
}

impl ToTaffy<taffy::LengthPercentage> for FixedLength {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::LengthPercentage {
        match self {
            FixedLength::Absolute(length) => {
                taffy::LengthPercentage::Length(length.to_pixels(rem_size).0)
            }
            FixedLength::Percent(percent) => taffy::LengthPercentage::Percent(*percent),
        }
    }
}

impl ToTaffy<taffy::Dimension> for FixedLength {
    fn to_taffy(&self, rem_size: Pixels) -> taffy::Dimension {
        match self {
            FixedLength::Absolute(length) => taffy::Dimension::Length(length.to_pixels(rem_size).0),
            FixedLength::Percent(percent) => taffy::Dimension::Percent(*percent),
        }
    }
}

impl<T> From<Edges<T>> for taffy::Rect<T> {
    fn from(value: Edges<T>) -> Self {
        taffy::Rect {
            top: value.top,
            right: value.right,
            bottom: value.bottom,
            left: value.left,
        }
    }
}
