use crate::{
    arc_string::ArcString,
    math::Corners,
    text_system::{Font, FontStyle, FontWeight},
    traits::Zero,
};
use std::fmt::Debug;

use crate::math::{Rect, Vec2};

use super::{path::Path2D, Color, TextureId};

#[derive(Debug, Clone)]
pub enum PrimitiveKind {
    Quad(Quad),
    Path(Path2D),
    Circle(Circle),
    Text(Text),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillStyle {
    pub color: Color,
}

impl Default for FillStyle {
    fn default() -> Self {
        Self {
            color: Color::TRANSPARENT,
        }
    }
}

impl FillStyle {
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineJoin {
    Miter,
    Bevel,
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineCap {
    Round,
    Square,
    Butt,
    Joint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeStyle {
    pub color: Color,
    pub line_width: u32,
    pub line_join: LineJoin,
    pub line_cap: LineCap,
    pub allow_overlap: bool,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            line_width: 2,
            line_join: LineJoin::Miter,
            line_cap: LineCap::Butt,
            allow_overlap: false,
        }
    }
}

impl StrokeStyle {
    pub fn allow_overlap(mut self, allow: bool) -> Self {
        self.allow_overlap = allow;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn line_width(mut self, line_width: u32) -> Self {
        self.line_width = line_width;
        self
    }

    pub fn line_join(mut self, line_join: LineJoin) -> Self {
        self.line_join = line_join;
        self
    }

    pub fn line_cap(mut self, line_cap: LineCap) -> Self {
        self.line_cap = line_cap;
        self
    }

    pub fn default_join(mut self) -> Self {
        self.line_join = LineJoin::Miter;
        self
    }

    pub fn miter_join(mut self) -> Self {
        self.line_join = LineJoin::Miter;
        self
    }

    pub fn bevel_join(mut self) -> Self {
        self.line_join = LineJoin::Bevel;
        self
    }

    pub fn round_join(mut self) -> Self {
        self.line_join = LineJoin::Round;
        self
    }

    pub fn round_cap(mut self) -> Self {
        self.line_cap = LineCap::Round;
        self
    }

    /// aka with_butt_join lol
    pub fn default_cap(mut self) -> Self {
        self.line_cap = LineCap::Butt;
        self
    }

    pub fn square_cap(mut self) -> Self {
        self.line_cap = LineCap::Square;
        self
    }

    /// sets line cap to join which will join the last point to first point
    pub fn join(mut self) -> Self {
        self.line_cap = LineCap::Joint;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub kind: PrimitiveKind,
    pub texture: TextureId,
    pub fill: FillStyle,
    pub stroke: Option<StrokeStyle>,
}

impl Primitive {
    #[inline(always)]
    pub(crate) fn can_render(&self) -> bool {
        let stroke_color = self.stroke.map_or(Color::TRANSPARENT, |s| s.color);
        !self.fill.color.is_transparent() || !stroke_color.is_transparent()
    }

    pub fn textured(mut self, tex: &TextureId) -> Self {
        self.texture = tex.clone();
        if self.fill.color.is_transparent() {
            self.fill.color = Color::WHITE;
        }
        self
    }

    pub fn no_fill(mut self) -> Self {
        self.fill.color = Color::TRANSPARENT;
        self
    }

    pub fn no_stroke(mut self) -> Self {
        self.stroke = None;
        self
    }

    pub fn fill(mut self, style: FillStyle) -> Self {
        self.fill = style;
        self
    }

    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill.color = color;
        self
    }

    pub fn stroke(mut self, style: StrokeStyle) -> Self {
        self.stroke.replace(style);
        self
    }

    pub fn stroke_color(mut self, color: Color) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.color = color;
        self
    }

    pub fn stroke_width(mut self, width: u32) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_width = width;
        self
    }

    pub fn line_join(mut self, join: LineJoin) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_join = join;
        self
    }

    pub fn line_cap(mut self, cap: LineCap) -> Self {
        let stroke = self.stroke.get_or_insert(Default::default());
        stroke.line_cap = cap;
        self
    }
}

#[derive(Debug, Default, Clone)]
pub struct Circle {
    pub center: Vec2<f32>,
    pub radius: f32,
}

impl Circle {
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn pos(mut self, cx: f32, cy: f32) -> Self {
        self.center.x = cx;
        self.center.y = cy;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Quad {
    pub bounds: Rect<f32>,
    pub corners: Corners<f32>,
}

impl Quad {
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.bounds.size.width = width;
        self.bounds.size.height = height;
        self
    }

    pub fn pos(mut self, x: f32, y: f32) -> Self {
        self.bounds.origin.x = x;
        self.bounds.origin.y = y;
        self
    }

    pub fn rect(mut self, rect: Rect<f32>) -> Self {
        self.bounds = rect;
        self
    }

    pub fn corners(mut self, corners: Corners<f32>) -> Self {
        self.corners = corners;
        self
    }
}

impl Default for Quad {
    fn default() -> Self {
        Self {
            bounds: Rect::zero(),
            corners: Corners::default(),
        }
    }
}

#[inline]
pub fn quad() -> Quad {
    Quad::default()
}

#[inline]
pub fn circle() -> Circle {
    Circle::default()
}

#[inline]
pub fn text(text: impl Into<ArcString>) -> Text {
    Text::new(text)
}

impl From<Quad> for PrimitiveKind {
    #[inline]
    fn from(quad: Quad) -> Self {
        PrimitiveKind::Quad(quad)
    }
}

impl From<Circle> for PrimitiveKind {
    #[inline]
    fn from(circle: Circle) -> Self {
        PrimitiveKind::Circle(circle)
    }
}

impl From<Path2D> for PrimitiveKind {
    #[inline]
    fn from(path: Path2D) -> Self {
        PrimitiveKind::Path(path)
    }
}

impl From<Text> for PrimitiveKind {
    #[inline]
    fn from(text: Text) -> Self {
        PrimitiveKind::Text(text)
    }
}

impl<T> From<T> for Primitive
where
    T: Into<PrimitiveKind>,
{
    fn from(value: T) -> Self {
        Primitive {
            kind: value.into(),
            texture: TextureId::WHITE_TEXTURE,
            fill: FillStyle::default(),
            stroke: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextBaseline {
    #[default]
    Alphabetic,
    Top,
    Hanging,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Right,
    Center,
}

#[derive(Debug, Clone)]
pub struct Text {
    pub(crate) text: ArcString,
    pub(crate) font: Font,
    pub(crate) pos: Vec2<f32>,
    pub(crate) align: TextAlign,
    pub(crate) word_spacing: f32,
    pub(crate) baseline: TextBaseline,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            pos: Vec2::zero(),
            text: ArcString::new_static(""),
            font: Font {
                family: ArcString::new_static("SegoueUI"),
                weight: FontWeight::default(),
                style: FontStyle::default(),
            },
            align: Default::default(),
            baseline: Default::default(),
            word_spacing: f32::zero(),
        }
    }
}

impl Text {
    pub fn new(text: impl Into<ArcString>) -> Self {
        Self::default().text(text.into())
    }

    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn baseline(mut self, baseline: TextBaseline) -> Self {
        self.baseline = baseline;
        self
    }

    pub fn pos(mut self, pos: Vec2<f32>) -> Self {
        self.pos = pos;
        self
    }

    pub fn text(mut self, text: ArcString) -> Self {
        self.text = text;
        self
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub fn font_family(mut self, font_family: impl Into<ArcString>) -> Self {
        self.font.family = font_family.into();
        self
    }

    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font.weight = font_weight;
        self
    }

    pub fn font_style(mut self, font_style: FontStyle) -> Self {
        self.font.style = font_style;
        self
    }

    pub fn word_spacing(mut self, spacing_in_px: f32) -> Self {
        self.word_spacing = spacing_in_px;
        self
    }
}

pub trait AsPrimitive {
    fn primitive(self) -> Primitive;
}

impl<T> AsPrimitive for T
where
    T: Into<Primitive>,
{
    fn primitive(self) -> Primitive {
        self.into()
    }
}

// macro_rules! impl_into_primitive {
//     ($t: ty, $kind: tt) => {
//         impl From<$t> for PrimitiveKind {
//             fn from(val: $t) -> Self {
//                 PrimitiveKind::$kind(val)
//             }
//         }
//     };
// }
//
