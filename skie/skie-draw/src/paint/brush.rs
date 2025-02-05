use crate::path::Contour;

use super::Color;

/// Represents a brush used for drawing operations, which includes properties for fill style, stroke style, and anti-aliasing.
#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub(crate) fill_style: FillStyle,
    pub(crate) stroke_style: StrokeStyle,
    pub(crate) antialias: bool,
}

impl Default for Brush {
    /// Creates a default brush with transparent fill and stroke, and anti-aliasing disabled.
    fn default() -> Self {
        Self {
            fill_style: FillStyle {
                color: Color::TRANSPARENT,
            },
            stroke_style: StrokeStyle {
                color: Color::TRANSPARENT,
                ..Default::default()
            },
            antialias: false,
        }
    }
}

impl Brush {
    pub fn filled(fill_color: Color) -> Self {
        Self {
            fill_style: FillStyle { color: fill_color },
            ..Default::default()
        }
    }
    /// Returns whether anti-aliasing is enabled for the brush.
    pub fn is_antialias(&self) -> bool {
        self.antialias
    }

    /// Enables or disables anti-aliasing for the brush.
    ///
    /// # Arguments
    ///
    /// * `enable` - A boolean value to enable (true) or disable (false) anti-aliasing.
    pub fn antialias(mut self, enable: bool) -> Self {
        self.antialias = enable;
        self
    }

    /// Gets the current fill color of the brush.
    pub fn get_fill_color(&self) -> Color {
        self.fill_style.color
    }

    /// Sets the fill color of the brush.
    ///
    /// # Arguments
    ///
    /// * `color` - The new fill color to be applied.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_style.color = color;
        self
    }

    pub fn reset_fill(mut self) -> Self {
        self.fill_style = Default::default();
        self
    }

    pub fn reset_stroke(mut self) -> Self {
        self.stroke_style = Default::default();
        self
    }

    pub fn no_fill(mut self) -> Self {
        self.fill_style.color = Color::TRANSPARENT;
        self
    }

    pub fn no_stroke(mut self) -> Self {
        self.stroke_style.color = Color::TRANSPARENT;
        self
    }

    /// Sets the fill style of the brush.
    ///
    /// # Arguments
    ///
    /// * `fill_style` - The new fill style (color and other properties).
    pub fn fill_style(mut self, fill_style: FillStyle) -> Self {
        self.fill_style = fill_style;
        self
    }

    /// Gets the current stroke color of the brush.
    pub fn get_stroke_color(&self) -> Color {
        self.fill_style.color
    }

    /// Sets the stroke color of the brush.
    ///
    /// # Arguments
    ///
    /// * `color` - The new stroke color to be applied.
    pub fn stroke_color(mut self, color: Color) -> Self {
        self.stroke_style.color = color;
        self
    }

    /// Sets the stroke width (line width) of the brush.
    ///
    /// # Arguments
    ///
    /// * `stroke_width` - The new stroke width to be applied.
    pub fn stroke_width(mut self, stroke_width: u32) -> Self {
        self.stroke_style.stroke_width = stroke_width;
        self
    }

    /// Sets the stroke style of the brush.
    ///
    /// # Arguments
    ///
    /// * `stroke_style` - The new stroke style (color, width, and other properties).
    pub fn stroke_style(mut self, stroke_style: StrokeStyle) -> Self {
        self.stroke_style = stroke_style;
        self
    }

    /// Sets the stroke line join style for the brush.
    ///
    /// # Arguments
    ///
    /// * `line_join` - The line join style (e.g., miter, round, bevel).
    pub fn stroke_join(mut self, stroke_join: StrokeJoin) -> Self {
        self.stroke_style.stroke_join = stroke_join;
        self
    }

    /// Sets the stroke line cap style for the brush.
    ///
    /// # Arguments
    ///
    /// * `line_cap` - The line cap style (e.g., butt, round, square).
    pub fn stroke_cap(mut self, stroke_cap: StrokeCap) -> Self {
        self.stroke_style.stroke_cap = stroke_cap;
        self
    }

    /// Resets the brush to its default state.
    pub fn reset(self) -> Self {
        Self::default()
    }

    /// Checks if there is nothing to draw with the brush (i.e., both the fill and stroke colors are transparent).
    pub fn noting_to_draw(&self) -> bool {
        self.fill_style.color.is_transparent() && self.stroke_style.color.is_transparent()
    }

    pub fn when<F>(self, cond: bool, consequent: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if cond {
            consequent(self)
        } else {
            self
        }
    }

    pub fn when_or<C, A>(self, cond: bool, consequent: C, alternate: A) -> Self
    where
        C: FnOnce(Self) -> Self,
        A: FnOnce(Self) -> Self,
    {
        if cond {
            consequent(self)
        } else {
            alternate(self)
        }
    }
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
pub enum StrokeJoin {
    Miter,
    Bevel,
    Round,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrokeCap {
    Round,
    Square,
    Butt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeStyle {
    pub color: Color,
    pub stroke_width: u32,
    pub stroke_join: StrokeJoin,
    pub stroke_cap: StrokeCap,
    pub allow_overlap: bool,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            stroke_width: 2,
            stroke_join: StrokeJoin::Miter,
            stroke_cap: StrokeCap::Butt,
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
        self.stroke_width = line_width;
        self
    }

    pub fn line_join(mut self, line_join: StrokeJoin) -> Self {
        self.stroke_join = line_join;
        self
    }

    pub fn line_cap(mut self, line_cap: StrokeCap) -> Self {
        self.stroke_cap = line_cap;
        self
    }

    pub fn default_join(mut self) -> Self {
        self.stroke_join = StrokeJoin::Miter;
        self
    }

    pub fn miter_join(mut self) -> Self {
        self.stroke_join = StrokeJoin::Miter;
        self
    }

    pub fn bevel_join(mut self) -> Self {
        self.stroke_join = StrokeJoin::Bevel;
        self
    }

    pub fn round_join(mut self) -> Self {
        self.stroke_join = StrokeJoin::Round;
        self
    }

    pub fn round_cap(mut self) -> Self {
        self.stroke_cap = StrokeCap::Round;
        self
    }

    /// aka with_butt_join lol
    pub fn default_cap(mut self) -> Self {
        self.stroke_cap = StrokeCap::Butt;
        self
    }

    pub fn square_cap(mut self) -> Self {
        self.stroke_cap = StrokeCap::Square;
        self
    }
}

#[derive(Debug, Clone)]
pub struct PathBrush {
    default: Brush,
    overrides: ahash::HashMap<Contour, Brush>,
}
impl PathBrush {
    pub fn new(default: Brush) -> Self {
        Self {
            default: default.clone(),
            ..Default::default()
        }
    }

    #[inline]
    pub fn set(&mut self, contour: Contour, brush: Brush) {
        self.overrides.insert(contour, brush);
    }

    #[inline]
    pub fn set_default(&mut self, default: Brush) {
        self.default = default;
    }

    #[inline]
    pub fn get_or_default(&self, contour: &Contour) -> Brush {
        self.overrides
            .get(contour)
            .cloned()
            .unwrap_or(self.default.clone())
    }
}

impl Default for PathBrush {
    fn default() -> Self {
        Self {
            default: Brush::filled(Color::WHITE),
            overrides: Default::default(),
        }
    }
}

impl From<Brush> for PathBrush {
    fn from(brush: Brush) -> Self {
        Self {
            default: brush,
            ..Default::default()
        }
    }
}

impl From<&Brush> for PathBrush {
    fn from(brush: &Brush) -> Self {
        Self {
            default: brush.clone(),
            ..Default::default()
        }
    }
}

impl<T> From<T> for PathBrush
where
    T: Iterator<Item = (Contour, Brush)>,
{
    fn from(value: T) -> Self {
        Self {
            default: Default::default(),
            overrides: value.collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use skie_math::vec2;

    use crate::{
        path::{PathBuilder, PathEventsIter, PathGeometryBuilder, Point},
        Color,
    };

    use super::{Brush, PathBrush};

    #[test]
    fn paint_brush_with_path() {
        let mut path = PathBuilder::default();

        let mut brush = PathBrush::default();

        let leg_paint = Brush::filled(Color::RED).stroke_width(10);

        path.begin(vec2(0.0, 0.0));
        path.line_to(vec2(-20.0, 100.0));
        let leg_l = path.end(false);
        brush.set(leg_l, leg_paint.clone());

        path.begin(vec2(0.0, 0.0));
        path.line_to(vec2(20.0, 100.0));
        let leg_r = path.end(false);
        brush.set(leg_r, leg_paint.clone());

        let head_paint = Brush::filled(Color::WHITE);
        let head = path.circle(vec2(0.0, 0.0), 10.0);
        brush.set(head, head_paint.clone());

        let mut output = <Vec<Point>>::new();

        let mut builder =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false)
                .map(|v| v.0);

        let leg_l_build = builder.next().expect("no contour");
        assert_eq!(leg_l, leg_l_build);

        let leg_r_build = builder.next().expect("no contour");
        assert_eq!(leg_r, leg_r_build);

        let head_build = builder.next().expect("no contour");
        assert_eq!(head, head_build);

        assert_eq!(builder.next(), None);

        assert_eq!(brush.get_or_default(&leg_l_build), leg_paint.clone());
        assert_eq!(brush.get_or_default(&leg_r_build), leg_paint);
        assert_eq!(brush.get_or_default(&head_build), head_paint);
    }
}
