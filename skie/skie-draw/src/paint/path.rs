use crate::{Corners, IsZero, Mat3, Rect, Vec2};

#[derive(Debug, Clone)]
pub struct Path2D {
    pub points: Vec<Vec2<f32>>,
    pub(crate) closed: bool,
    cursor: Vec2<f32>,
    start: Option<Vec2<f32>>,
    segment_count: u8,
}

impl Default for Path2D {
    fn default() -> Self {
        Self {
            points: Default::default(),
            segment_count: 32,
            closed: false,
            cursor: Default::default(),
            start: None,
        }
    }
}

impl Path2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    pub fn bounds(&self) -> Rect<f32> {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;

        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for point in &self.points {
            let x = point.x;
            let y = point.y;
            min_x = if x < min_x { x } else { min_x };
            max_x = if x > max_x { x } else { max_x };

            min_y = if y < min_y { y } else { min_y };
            max_y = if y > max_y { y } else { max_y };
        }

        Rect::from_corners((min_x, min_y).into(), (max_x, max_y).into())
    }

    pub fn segment_count(&self) -> u8 {
        self.segment_count
    }

    pub fn set_segment_count(&mut self, count: u8) {
        self.segment_count = count
    }

    pub fn with_segment_count<R>(&mut self, f: impl FnOnce(&mut Self) -> R, count: u8) -> R {
        let tmp = self.segment_count;
        self.segment_count = count;
        let res = f(self);
        self.segment_count = tmp;
        res
    }

    pub fn extend(&mut self, _path: &Path2D) {
        todo!()
    }

    /// Moves the cursor to the specified position without creating a line.
    #[inline]
    pub fn move_to(&mut self, new_pos: Vec2<f32>) {
        self.cursor = new_pos;
        self.start = Some(new_pos); // Set the start of the new subpath
    }

    /// Draws a straight line from the cursor to the specified position.
    pub fn line_to(&mut self, to: Vec2<f32>) {
        if let Some(start) = self.start {
            if self.cursor == start {
                self.points.push(self.cursor);
            }
        }

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
        self.closed = false;
    }

    /// Closes the current subpath by drawing a line back to the starting point.
    pub fn close(&mut self) {
        self.closed = true;
        // if let Some(start) = &self.start {
        //     self.line_to(*start);
        // }
    }

    pub fn circle(&mut self, center: Vec2<f32>, radius: f32) {
        use baked_verts::{CIRCLE_128, CIRCLE_16, CIRCLE_32, CIRCLE_64, CIRCLE_8};
        if radius <= 2.0 {
            self.points
                .extend(CIRCLE_8.iter().map(|&n| center + n * radius));
        } else if radius <= 5.0 {
            self.points
                .extend(CIRCLE_16.iter().map(|&n| center + n * radius));
        } else if radius < 18.0 {
            self.points
                .extend(CIRCLE_32.iter().map(|&n| center + n * radius));
        } else if radius < 50.0 {
            self.points
                .extend(CIRCLE_64.iter().map(|&n| center + n * radius));
        } else {
            self.points
                .extend(CIRCLE_128.iter().map(|&n| center + n * radius));
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
        // TODO: add the cursor ?
        // TODO: auto select segement count based on the radius

        let num_segments = self.segment_count;
        let step: f32 = if clockwise {
            -(end_angle - start_angle) / num_segments as f32
        } else {
            (end_angle - start_angle) / num_segments as f32
        };

        self.points.reserve(num_segments as usize);

        for i in 0..=num_segments {
            let theta = start_angle + i as f32 * step;
            let x = center.x + radius * theta.cos();
            let y = center.y + radius * theta.sin();
            let p = Vec2 { x, y };
            self.points.push(p);
        }

        // Update the cursor to the final point on the arc.
        if let Some(last) = self.points.last() {
            self.cursor = *last;
        }
    }

    pub fn rect(&mut self, rect: &Rect<f32>) {
        self.points.reserve(4);

        self.points.push(rect.top_left());
        self.points.push(rect.top_right());
        self.points.push(rect.bottom_right());
        self.points.push(rect.bottom_left());

        if let Some(last) = self.points.last() {
            self.cursor = *last;
        }
    }

    pub fn round_rect(&mut self, rect: &Rect<f32>, corners: &Corners<f32>) {
        if corners.is_zero() {
            self.rect(rect);
            return;
        }

        let Corners {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        } = corners;

        if corners.is_zero() {
            self.rect(rect);
            return;
        }

        const PI: f32 = std::f32::consts::PI;

        // TODO: auto select segment count
        let segcount = self.segment_count();
        self.set_segment_count(16);

        // top-left-corner
        // self.move_to((rect.x(), rect.y() + top_left).into());
        self.arc(
            (rect.x() + top_left, rect.y() + top_left).into(),
            *top_left,
            PI,
            (3.0 * PI) / 2.0,
            false,
        );

        // top-right-corner
        // self.move_to((rect.x() + top_right, rect.y() + rect.height()).into());
        self.arc(
            (rect.x() + rect.width() - top_right, rect.y() + top_right).into(),
            *top_right,
            -PI / 2.0,
            0.0,
            false,
        );

        // bottom-right-corner
        // self.move_to((rect.x + rect.width, rect.y + rect.height - bottom_right).into());
        self.arc(
            (
                rect.x() + rect.width() - bottom_right,
                rect.y() + rect.height() - bottom_right,
            )
                .into(),
            *bottom_right,
            0.0,
            PI / 2.0,
            false,
        );

        // bottom-left-corner
        // self.move_to((rect.x + bottom_left, rect.y + rect.height).into());
        self.arc(
            (
                rect.x() + bottom_left,
                rect.y() + rect.height() - bottom_left,
            )
                .into(),
            *bottom_left,
            PI / 2.0,
            PI,
            false,
        );

        self.set_segment_count(segcount);
    }
}

impl std::ops::Mul<&mut Path2D> for Mat3 {
    type Output = ();
    fn mul(self, path: &mut Path2D) -> Self::Output {
        for point in path.points.iter_mut() {
            *point = self * (*point)
        }
    }
}

#[allow(clippy::approx_constant)]
mod baked_verts {

    // fn gen(n: usize) {
    //     println!("#[rustfmt::skip]");
    //     println!("pub const CIRCLE_{}: [Vec2<f32>; {}] = [", n, n + 1);
    //     for i in 0..=n {
    //         let a = std::f64::consts::TAU * i as f64 / n as f64;
    //         println!("    Vec2 {{  x: {:.06}, y: {:.06} }},", a.cos(), a.sin());
    //     }
    //     println!("];");
    //     println!("\n");
    // }
    //
    // fn main() {
    //     gen(8);
    //     gen(16);
    //     gen(32);
    //     gen(64);
    //     gen(128);
    // }

    use crate::math::Vec2;

    #[rustfmt::skip]
    pub const CIRCLE_8: [Vec2<f32>; 9] = [
        Vec2 {  x: 1.000000, y: 0.000000 },
        Vec2 {  x: 0.707107, y: 0.707107 },
        Vec2 {  x: 0.000000, y: 1.000000 },
        Vec2 {  x: -0.707107, y: 0.707107 },
        Vec2 {  x: -1.000000, y: 0.000000 },
        Vec2 {  x: -0.707107, y: -0.707107 },
        Vec2 {  x: -0.000000, y: -1.000000 },
        Vec2 {  x: 0.707107, y: -0.707107 },
        Vec2 {  x: 1.000000, y: -0.000000 },
    ];

    #[rustfmt::skip]
    pub const CIRCLE_16: [Vec2<f32>; 17] = [
        Vec2 {  x: 1.000000, y: 0.000000 },
        Vec2 {  x: 0.923880, y: 0.382683 },
        Vec2 {  x: 0.707107, y: 0.707107 },
        Vec2 {  x: 0.382683, y: 0.923880 },
        Vec2 {  x: 0.000000, y: 1.000000 },
        Vec2 {  x: -0.382683, y: 0.923880 },
        Vec2 {  x: -0.707107, y: 0.707107 },
        Vec2 {  x: -0.923880, y: 0.382683 },
        Vec2 {  x: -1.000000, y: 0.000000 },
        Vec2 {  x: -0.923880, y: -0.382683 },
        Vec2 {  x: -0.707107, y: -0.707107 },
        Vec2 {  x: -0.382683, y: -0.923880 },
        Vec2 {  x: -0.000000, y: -1.000000 },
        Vec2 {  x: 0.382683, y: -0.923880 },
        Vec2 {  x: 0.707107, y: -0.707107 },
        Vec2 {  x: 0.923880, y: -0.382683 },
        Vec2 {  x: 1.000000, y: -0.000000 },
    ];

    #[rustfmt::skip]
    pub const CIRCLE_32: [Vec2<f32>; 33] = [
        Vec2 {  x: 1.000000, y: 0.000000 },
        Vec2 {  x: 0.980785, y: 0.195090 },
        Vec2 {  x: 0.923880, y: 0.382683 },
        Vec2 {  x: 0.831470, y: 0.555570 },
        Vec2 {  x: 0.707107, y: 0.707107 },
        Vec2 {  x: 0.555570, y: 0.831470 },
        Vec2 {  x: 0.382683, y: 0.923880 },
        Vec2 {  x: 0.195090, y: 0.980785 },
        Vec2 {  x: 0.000000, y: 1.000000 },
        Vec2 {  x: -0.195090, y: 0.980785 },
        Vec2 {  x: -0.382683, y: 0.923880 },
        Vec2 {  x: -0.555570, y: 0.831470 },
        Vec2 {  x: -0.707107, y: 0.707107 },
        Vec2 {  x: -0.831470, y: 0.555570 },
        Vec2 {  x: -0.923880, y: 0.382683 },
        Vec2 {  x: -0.980785, y: 0.195090 },
        Vec2 {  x: -1.000000, y: 0.000000 },
        Vec2 {  x: -0.980785, y: -0.195090 },
        Vec2 {  x: -0.923880, y: -0.382683 },
        Vec2 {  x: -0.831470, y: -0.555570 },
        Vec2 {  x: -0.707107, y: -0.707107 },
        Vec2 {  x: -0.555570, y: -0.831470 },
        Vec2 {  x: -0.382683, y: -0.923880 },
        Vec2 {  x: -0.195090, y: -0.980785 },
        Vec2 {  x: -0.000000, y: -1.000000 },
        Vec2 {  x: 0.195090, y: -0.980785 },
        Vec2 {  x: 0.382683, y: -0.923880 },
        Vec2 {  x: 0.555570, y: -0.831470 },
        Vec2 {  x: 0.707107, y: -0.707107 },
        Vec2 {  x: 0.831470, y: -0.555570 },
        Vec2 {  x: 0.923880, y: -0.382683 },
        Vec2 {  x: 0.980785, y: -0.195090 },
        Vec2 {  x: 1.000000, y: -0.000000 },
    ];

    #[rustfmt::skip]
    pub const CIRCLE_64: [Vec2<f32>; 65] = [
        Vec2 {  x: 1.000000, y: 0.000000 },
        Vec2 {  x: 0.995185, y: 0.098017 },
        Vec2 {  x: 0.980785, y: 0.195090 },
        Vec2 {  x: 0.956940, y: 0.290285 },
        Vec2 {  x: 0.923880, y: 0.382683 },
        Vec2 {  x: 0.881921, y: 0.471397 },
        Vec2 {  x: 0.831470, y: 0.555570 },
        Vec2 {  x: 0.773010, y: 0.634393 },
        Vec2 {  x: 0.707107, y: 0.707107 },
        Vec2 {  x: 0.634393, y: 0.773010 },
        Vec2 {  x: 0.555570, y: 0.831470 },
        Vec2 {  x: 0.471397, y: 0.881921 },
        Vec2 {  x: 0.382683, y: 0.923880 },
        Vec2 {  x: 0.290285, y: 0.956940 },
        Vec2 {  x: 0.195090, y: 0.980785 },
        Vec2 {  x: 0.098017, y: 0.995185 },
        Vec2 {  x: 0.000000, y: 1.000000 },
        Vec2 {  x: -0.098017, y: 0.995185 },
        Vec2 {  x: -0.195090, y: 0.980785 },
        Vec2 {  x: -0.290285, y: 0.956940 },
        Vec2 {  x: -0.382683, y: 0.923880 },
        Vec2 {  x: -0.471397, y: 0.881921 },
        Vec2 {  x: -0.555570, y: 0.831470 },
        Vec2 {  x: -0.634393, y: 0.773010 },
        Vec2 {  x: -0.707107, y: 0.707107 },
        Vec2 {  x: -0.773010, y: 0.634393 },
        Vec2 {  x: -0.831470, y: 0.555570 },
        Vec2 {  x: -0.881921, y: 0.471397 },
        Vec2 {  x: -0.923880, y: 0.382683 },
        Vec2 {  x: -0.956940, y: 0.290285 },
        Vec2 {  x: -0.980785, y: 0.195090 },
        Vec2 {  x: -0.995185, y: 0.098017 },
        Vec2 {  x: -1.000000, y: 0.000000 },
        Vec2 {  x: -0.995185, y: -0.098017 },
        Vec2 {  x: -0.980785, y: -0.195090 },
        Vec2 {  x: -0.956940, y: -0.290285 },
        Vec2 {  x: -0.923880, y: -0.382683 },
        Vec2 {  x: -0.881921, y: -0.471397 },
        Vec2 {  x: -0.831470, y: -0.555570 },
        Vec2 {  x: -0.773010, y: -0.634393 },
        Vec2 {  x: -0.707107, y: -0.707107 },
        Vec2 {  x: -0.634393, y: -0.773010 },
        Vec2 {  x: -0.555570, y: -0.831470 },
        Vec2 {  x: -0.471397, y: -0.881921 },
        Vec2 {  x: -0.382683, y: -0.923880 },
        Vec2 {  x: -0.290285, y: -0.956940 },
        Vec2 {  x: -0.195090, y: -0.980785 },
        Vec2 {  x: -0.098017, y: -0.995185 },
        Vec2 {  x: -0.000000, y: -1.000000 },
        Vec2 {  x: 0.098017, y: -0.995185 },
        Vec2 {  x: 0.195090, y: -0.980785 },
        Vec2 {  x: 0.290285, y: -0.956940 },
        Vec2 {  x: 0.382683, y: -0.923880 },
        Vec2 {  x: 0.471397, y: -0.881921 },
        Vec2 {  x: 0.555570, y: -0.831470 },
        Vec2 {  x: 0.634393, y: -0.773010 },
        Vec2 {  x: 0.707107, y: -0.707107 },
        Vec2 {  x: 0.773010, y: -0.634393 },
        Vec2 {  x: 0.831470, y: -0.555570 },
        Vec2 {  x: 0.881921, y: -0.471397 },
        Vec2 {  x: 0.923880, y: -0.382683 },
        Vec2 {  x: 0.956940, y: -0.290285 },
        Vec2 {  x: 0.980785, y: -0.195090 },
        Vec2 {  x: 0.995185, y: -0.098017 },
        Vec2 {  x: 1.000000, y: -0.000000 },
    ];

    #[rustfmt::skip]
pub const CIRCLE_128: [Vec2<f32>; 129] = [
    Vec2 {  x: 1.000000, y: 0.000000 },
    Vec2 {  x: 0.998795, y: 0.049068 },
    Vec2 {  x: 0.995185, y: 0.098017 },
    Vec2 {  x: 0.989177, y: 0.146730 },
    Vec2 {  x: 0.980785, y: 0.195090 },
    Vec2 {  x: 0.970031, y: 0.242980 },
    Vec2 {  x: 0.956940, y: 0.290285 },
    Vec2 {  x: 0.941544, y: 0.336890 },
    Vec2 {  x: 0.923880, y: 0.382683 },
    Vec2 {  x: 0.903989, y: 0.427555 },
    Vec2 {  x: 0.881921, y: 0.471397 },
    Vec2 {  x: 0.857729, y: 0.514103 },
    Vec2 {  x: 0.831470, y: 0.555570 },
    Vec2 {  x: 0.803208, y: 0.595699 },
    Vec2 {  x: 0.773010, y: 0.634393 },
    Vec2 {  x: 0.740951, y: 0.671559 },
    Vec2 {  x: 0.707107, y: 0.707107 },
    Vec2 {  x: 0.671559, y: 0.740951 },
    Vec2 {  x: 0.634393, y: 0.773010 },
    Vec2 {  x: 0.595699, y: 0.803208 },
    Vec2 {  x: 0.555570, y: 0.831470 },
    Vec2 {  x: 0.514103, y: 0.857729 },
    Vec2 {  x: 0.471397, y: 0.881921 },
    Vec2 {  x: 0.427555, y: 0.903989 },
    Vec2 {  x: 0.382683, y: 0.923880 },
    Vec2 {  x: 0.336890, y: 0.941544 },
    Vec2 {  x: 0.290285, y: 0.956940 },
    Vec2 {  x: 0.242980, y: 0.970031 },
    Vec2 {  x: 0.195090, y: 0.980785 },
    Vec2 {  x: 0.146730, y: 0.989177 },
    Vec2 {  x: 0.098017, y: 0.995185 },
    Vec2 {  x: 0.049068, y: 0.998795 },
    Vec2 {  x: 0.000000, y: 1.000000 },
    Vec2 {  x: -0.049068, y: 0.998795 },
    Vec2 {  x: -0.098017, y: 0.995185 },
    Vec2 {  x: -0.146730, y: 0.989177 },
    Vec2 {  x: -0.195090, y: 0.980785 },
    Vec2 {  x: -0.242980, y: 0.970031 },
    Vec2 {  x: -0.290285, y: 0.956940 },
    Vec2 {  x: -0.336890, y: 0.941544 },
    Vec2 {  x: -0.382683, y: 0.923880 },
    Vec2 {  x: -0.427555, y: 0.903989 },
    Vec2 {  x: -0.471397, y: 0.881921 },
    Vec2 {  x: -0.514103, y: 0.857729 },
    Vec2 {  x: -0.555570, y: 0.831470 },
    Vec2 {  x: -0.595699, y: 0.803208 },
    Vec2 {  x: -0.634393, y: 0.773010 },
    Vec2 {  x: -0.671559, y: 0.740951 },
    Vec2 {  x: -0.707107, y: 0.707107 },
    Vec2 {  x: -0.740951, y: 0.671559 },
    Vec2 {  x: -0.773010, y: 0.634393 },
    Vec2 {  x: -0.803208, y: 0.595699 },
    Vec2 {  x: -0.831470, y: 0.555570 },
    Vec2 {  x: -0.857729, y: 0.514103 },
    Vec2 {  x: -0.881921, y: 0.471397 },
    Vec2 {  x: -0.903989, y: 0.427555 },
    Vec2 {  x: -0.923880, y: 0.382683 },
    Vec2 {  x: -0.941544, y: 0.336890 },
    Vec2 {  x: -0.956940, y: 0.290285 },
    Vec2 {  x: -0.970031, y: 0.242980 },
    Vec2 {  x: -0.980785, y: 0.195090 },
    Vec2 {  x: -0.989177, y: 0.146730 },
    Vec2 {  x: -0.995185, y: 0.098017 },
    Vec2 {  x: -0.998795, y: 0.049068 },
    Vec2 {  x: -1.000000, y: 0.000000 },
    Vec2 {  x: -0.998795, y: -0.049068 },
    Vec2 {  x: -0.995185, y: -0.098017 },
    Vec2 {  x: -0.989177, y: -0.146730 },
    Vec2 {  x: -0.980785, y: -0.195090 },
    Vec2 {  x: -0.970031, y: -0.242980 },
    Vec2 {  x: -0.956940, y: -0.290285 },
    Vec2 {  x: -0.941544, y: -0.336890 },
    Vec2 {  x: -0.923880, y: -0.382683 },
    Vec2 {  x: -0.903989, y: -0.427555 },
    Vec2 {  x: -0.881921, y: -0.471397 },
    Vec2 {  x: -0.857729, y: -0.514103 },
    Vec2 {  x: -0.831470, y: -0.555570 },
    Vec2 {  x: -0.803208, y: -0.595699 },
    Vec2 {  x: -0.773010, y: -0.634393 },
    Vec2 {  x: -0.740951, y: -0.671559 },
    Vec2 {  x: -0.707107, y: -0.707107 },
    Vec2 {  x: -0.671559, y: -0.740951 },
    Vec2 {  x: -0.634393, y: -0.773010 },
    Vec2 {  x: -0.595699, y: -0.803208 },
    Vec2 {  x: -0.555570, y: -0.831470 },
    Vec2 {  x: -0.514103, y: -0.857729 },
    Vec2 {  x: -0.471397, y: -0.881921 },
    Vec2 {  x: -0.427555, y: -0.903989 },
    Vec2 {  x: -0.382683, y: -0.923880 },
    Vec2 {  x: -0.336890, y: -0.941544 },
    Vec2 {  x: -0.290285, y: -0.956940 },
    Vec2 {  x: -0.242980, y: -0.970031 },
    Vec2 {  x: -0.195090, y: -0.980785 },
    Vec2 {  x: -0.146730, y: -0.989177 },
    Vec2 {  x: -0.098017, y: -0.995185 },
    Vec2 {  x: -0.049068, y: -0.998795 },
    Vec2 {  x: -0.000000, y: -1.000000 },
    Vec2 {  x: 0.049068, y: -0.998795 },
    Vec2 {  x: 0.098017, y: -0.995185 },
    Vec2 {  x: 0.146730, y: -0.989177 },
    Vec2 {  x: 0.195090, y: -0.980785 },
    Vec2 {  x: 0.242980, y: -0.970031 },
    Vec2 {  x: 0.290285, y: -0.956940 },
    Vec2 {  x: 0.336890, y: -0.941544 },
    Vec2 {  x: 0.382683, y: -0.923880 },
    Vec2 {  x: 0.427555, y: -0.903989 },
    Vec2 {  x: 0.471397, y: -0.881921 },
    Vec2 {  x: 0.514103, y: -0.857729 },
    Vec2 {  x: 0.555570, y: -0.831470 },
    Vec2 {  x: 0.595699, y: -0.803208 },
    Vec2 {  x: 0.634393, y: -0.773010 },
    Vec2 {  x: 0.671559, y: -0.740951 },
    Vec2 {  x: 0.707107, y: -0.707107 },
    Vec2 {  x: 0.740951, y: -0.671559 },
    Vec2 {  x: 0.773010, y: -0.634393 },
    Vec2 {  x: 0.803208, y: -0.595699 },
    Vec2 {  x: 0.831470, y: -0.555570 },
    Vec2 {  x: 0.857729, y: -0.514103 },
    Vec2 {  x: 0.881921, y: -0.471397 },
    Vec2 {  x: 0.903989, y: -0.427555 },
    Vec2 {  x: 0.923880, y: -0.382683 },
    Vec2 {  x: 0.941544, y: -0.336890 },
    Vec2 {  x: 0.956940, y: -0.290285 },
    Vec2 {  x: 0.970031, y: -0.242980 },
    Vec2 {  x: 0.980785, y: -0.195090 },
    Vec2 {  x: 0.989177, y: -0.146730 },
    Vec2 {  x: 0.995185, y: -0.098017 },
    Vec2 {  x: 0.998795, y: -0.049068 },
    Vec2 {  x: 1.000000, y: -0.000000 },
];
}

#[cfg(test)]
mod tests {
    // TODO:  more tests are needed
    use crate::math::vec2;

    use super::*;
    #[test]
    fn test_line_to_multiple_lines() {
        // Create a new GeometryPath
        let mut path = Path2D::new();

        path.move_to(vec2(50.0, 50.0));
        path.line_to(vec2(100.0, 100.0));
        path.line_to(vec2(150.0, 150.0));
        path.line_to(vec2(200.0, 300.0));

        path.move_to(vec2(500.0, 500.0));
        path.line_to(vec2(100.0, 100.0));

        let expected = vec![
            vec2(50.0, 50.0),   // Start position
            vec2(100.0, 100.0), // First line
            vec2(150.0, 150.0), // Second line
            vec2(200.0, 300.0), // Second line
            vec2(500.0, 500.0),
            vec2(100.0, 100.0),
        ];
        assert_eq!(path.points, expected);
    }

    #[test]
    fn basic_path_test() {
        let expected: &[Vec2<f32>] = &[
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

        path.arc(
            (300.0, 300.0).into(),
            100.0,
            0.0,
            std::f32::consts::PI,
            false,
        );

        assert_eq!(expected, path.points);
    }
}
