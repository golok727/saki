use super::Color;

/// Represents a brush used for drawing operations, which includes properties for fill style, stroke style, and anti-aliasing.
#[derive(Debug, Clone)]
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
    pub fn antialias(&mut self, enable: bool) {
        self.antialias = enable
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
    pub fn fill_color(&mut self, color: Color) {
        self.fill_style.color = color;
    }

    pub fn reset_fill(&mut self) {
        self.fill_style = Default::default();
    }

    pub fn reset_stroke(&mut self) {
        self.stroke_style = Default::default();
    }

    pub fn no_fill(&mut self) {
        self.fill_style.color = Color::TRANSPARENT;
    }

    pub fn no_stroke(&mut self) {
        self.stroke_style.color = Color::TRANSPARENT;
    }

    /// Sets the fill style of the brush.
    ///
    /// # Arguments
    ///
    /// * `fill_style` - The new fill style (color and other properties).
    pub fn fill_style(&mut self, fill_style: FillStyle) {
        self.fill_style = fill_style;
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
    pub fn stroke_color(&mut self, color: Color) {
        self.stroke_style.color = color;
    }

    /// Sets the stroke width (line width) of the brush.
    ///
    /// # Arguments
    ///
    /// * `stroke_width` - The new stroke width to be applied.
    pub fn stroke_width(&mut self, stroke_width: u32) {
        self.stroke_style.line_width = stroke_width;
    }

    /// Sets the stroke style of the brush.
    ///
    /// # Arguments
    ///
    /// * `stroke_style` - The new stroke style (color, width, and other properties).
    pub fn stroke_style(&mut self, stroke_style: StrokeStyle) {
        self.stroke_style = stroke_style;
    }

    /// Sets the stroke line join style for the brush.
    ///
    /// # Arguments
    ///
    /// * `line_join` - The line join style (e.g., miter, round, bevel).
    pub fn stroke_join(&mut self, line_join: LineJoin) {
        self.stroke_style.line_join = line_join;
    }

    /// Sets the stroke line cap style for the brush.
    ///
    /// # Arguments
    ///
    /// * `line_cap` - The line cap style (e.g., butt, round, square).
    pub fn stroke_cap(&mut self, line_cap: LineCap) {
        self.stroke_style.line_cap = line_cap;
    }

    /// Resets the brush to its default state.
    pub fn reset(&mut self) {
        *self = Self::default()
    }

    /// Checks if there is nothing to draw with the brush (i.e., both the fill and stroke colors are transparent).
    pub fn noting_to_draw(&self) -> bool {
        self.fill_style.color.is_transparent() && self.stroke_style.color.is_transparent()
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
