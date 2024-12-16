use crate::math::Vec2;

use super::{Path2D, PathOp};

#[derive(Default, Debug, Clone)]
pub struct GeometryPath {
    pub points: Vec<Vec2<f32>>,
    cursor: Vec2<f32>,
    start: Option<Vec2<f32>>,
}

impl GeometryPath {
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the cursor to the specified position without creating a line.
    pub fn move_to(&mut self, new_pos: Vec2<f32>) {
        self.cursor = new_pos;
        self.start = Some(new_pos); // Set the start of the new subpath
    }

    /// Draws a straight line from the cursor to the specified position.
    pub fn line_to(&mut self, to: Vec2<f32>) {
        self.points.push(self.cursor);
        self.points.push(to);
        self.cursor = to;
    }

    /// Draws a quadratic Bézier curve from the cursor to `to` using `control` as the control point.
    pub fn quadratic_bezier_to(&mut self, _control: Vec2<f32>, _to: Vec2<f32>) {
        todo!()
    }

    /// Clears all points in the path.
    pub fn clear(&mut self) {
        self.points.clear();
        self.start = None;
    }

    /// Closes the current subpath by drawing a line back to the starting point.
    pub fn close_path(&mut self) {
        if let Some(start) = &self.start {
            self.line_to(*start);
        }
    }

    /// Draws an arc.
    pub fn arc(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        clockwise: bool,
    ) {
        // TODO: auto select segement count based on the radius
        const NUM_SEGMENTS: u8 = 32;

        let step: f32 = if clockwise {
            -(end_angle - start_angle) / NUM_SEGMENTS as f32
        } else {
            (end_angle - start_angle) / NUM_SEGMENTS as f32
        };

        self.points.reserve(NUM_SEGMENTS as usize);

        for i in 0..=NUM_SEGMENTS {
            let theta = start_angle + i as f32 * step;
            let x = center.x + radius * theta.cos();
            let y = center.y + radius * theta.sin();
            self.points.push(Vec2 { x, y });
        }

        // Update the cursor to the final point on the arc.
        if let Some(last) = self.points.last() {
            self.cursor = *last;
        }
    }

    fn from_ops(ops: &[PathOp]) -> Self {
        let mut builder = Self::default();
        for op in ops {
            match op {
                PathOp::MoveTo(to) => builder.line_to(*to),
                PathOp::LineTo(to) => builder.line_to(*to),
                PathOp::QuadratcBezierTo { control, to } => {
                    builder.quadratic_bezier_to(*control, *to)
                }
                PathOp::ArcTo {
                    center,
                    radius,
                    start_angle,
                    end_angle,
                    clockwise,
                } => builder.arc(*center, *radius, *start_angle, *end_angle, *clockwise),
                PathOp::ClosePath => builder.close_path(),
            }
        }

        builder
    }
}

impl From<&Path2D> for GeometryPath {
    fn from(path: &Path2D) -> Self {
        Self::from_ops(&path.ops)
    }
}

impl From<Path2D> for GeometryPath {
    fn from(path: Path2D) -> Self {
        Self::from_ops(&path.ops)
    }
}

impl From<&[PathOp]> for GeometryPath {
    fn from(ops: &[PathOp]) -> Self {
        Self::from_ops(ops)
    }
}

#[cfg(test)]
mod tests {
    // TODO:  more tests are needed
    use crate::math::vec2;

    use super::*;
    #[test]
    fn basic_path_test() {
        let expected: &[Vec2<f32>] = &[
            vec2(0.0, 0.0),
            vec2(100.0, 100.0),
            vec2(100.0, 100.0),
            vec2(200.0, 300.0),
            vec2(400.0, 300.0),
            vec2(399.51846, 309.80173),
            vec2(398.07852, 319.50903),
            vec2(395.69403, 329.02847),
            vec2(392.38794, 338.26834),
            vec2(388.19214, 347.13968),
            vec2(383.14697, 355.55704),
            vec2(377.30103, 363.43933),
            vec2(370.7107, 370.7107),
            vec2(363.43933, 377.30103),
            vec2(355.557, 383.14697),
            vec2(347.13965, 388.19214),
            vec2(338.26834, 392.38794),
            vec2(329.02847, 395.69403),
            vec2(319.50903, 398.07852),
            vec2(309.80173, 399.51846),
            vec2(300.0, 400.0),
            vec2(290.19827, 399.51846),
            vec2(280.49097, 398.07852),
            vec2(270.97153, 395.69403),
            vec2(261.73166, 392.38794),
            vec2(252.86032, 388.19214),
            vec2(244.44296, 383.14697),
            vec2(236.56067, 377.30106),
            vec2(229.28932, 370.7107),
            vec2(222.69894, 363.43933),
            vec2(216.85303, 355.557),
            vec2(211.80786, 347.13965),
            vec2(207.61203, 338.2683),
            vec2(204.30597, 329.02847),
            vec2(201.92148, 319.50903),
            vec2(200.48154, 309.8017),
            vec2(200.0, 300.0),
        ];

        let mut path = Path2D::default();
        path.move_to((100.0, 100.0).into());
        path.line_to((200.0, 300.0).into());

        path.arc(
            (300.0, 300.0).into(),
            100.0,
            0.0,
            std::f32::consts::PI,
            false,
        );

        let p = GeometryPath::from(&path);
        assert_eq!(expected, p.points);
    }
}
