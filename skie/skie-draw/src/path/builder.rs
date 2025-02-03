use skie_math::{vec2, Corners, Rect};

use super::{Point, Polygon};

pub trait PathBuilder {
    // begin a subpath
    fn begin(&mut self, to: Point);

    // end a subpath
    fn end(&mut self, close: bool);

    fn close(&mut self) {
        self.end(true)
    }

    fn reserve(&mut self, endpoints: usize, ctrl_points: usize);

    fn line_to(&mut self, to: Point);

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point);

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point);

    fn add_point(&mut self, at: Point) {
        self.begin(at);
        self.end(false);
    }

    fn polygon(&mut self, polygon: Polygon<Point>) {
        if polygon.points.is_empty() {
            return;
        }

        self.reserve(polygon.points.len(), 0);

        self.begin(polygon.points[0]);

        for p in &polygon.points[1..] {
            self.line_to(*p);
        }

        self.end(polygon.closed);
    }

    fn rect(&mut self, rect: &Rect<f32>) {
        self.polygon(Polygon {
            points: &[
                rect.top_left(),
                rect.top_right(),
                rect.bottom_right(),
                rect.bottom_left(),
            ],
            closed: true,
        });
    }

    fn circle(&mut self, center: Point, radius: f32)
    where
        Self: Sized,
    {
        add_circle(self, center, radius);
    }

    fn round_rect(&mut self, rect: &Rect<f32>, corners: &Corners<f32>)
    where
        Self: Sized,
    {
        add_rounded_rectangle(self, rect, corners)
    }
}

fn add_circle<Builder: PathBuilder>(builder: &mut Builder, center: Point, radius: f32) {
    let radius = radius.abs();
    // let dir = match winding {
    //     Winding::Positive => 1.0,
    //     Winding::Negative => -1.0,
    // };

    let dir = 1.0;

    // https://spencermortensen.com/articles/bezier-circle/
    const CONSTANT_FACTOR: f32 = 0.55191505;
    let d = radius * CONSTANT_FACTOR;

    builder.begin(center + vec2(-radius, 0.0));

    let ctrl_0 = center + vec2(-radius, -d * dir);
    let ctrl_1 = center + vec2(-d, -radius * dir);
    let mid = center + vec2(0.0, -radius * dir);
    builder.cubic_bezier_to(ctrl_0, ctrl_1, mid);

    let ctrl_0 = center + vec2(d, -radius * dir);
    let ctrl_1 = center + vec2(radius, -d * dir);
    let mid = center + vec2(radius, 0.0);
    builder.cubic_bezier_to(ctrl_0, ctrl_1, mid);

    let ctrl_0 = center + vec2(radius, d * dir);
    let ctrl_1 = center + vec2(d, radius * dir);
    let mid = center + vec2(0.0, radius * dir);
    builder.cubic_bezier_to(ctrl_0, ctrl_1, mid);

    let ctrl_0 = center + vec2(-d, radius * dir);
    let ctrl_1 = center + vec2(-radius, d * dir);
    let mid = center + vec2(-radius, 0.0);
    builder.cubic_bezier_to(ctrl_0, ctrl_1, mid);

    builder.close();
}

fn add_rounded_rectangle<Builder: PathBuilder>(
    builder: &mut Builder,
    rect: &Rect<f32>,
    corners: &Corners<f32>,
) {
    let w = rect.size.width;
    let h = rect.size.height;
    let min = rect.min();
    let max = rect.max();

    let x_min = min.x;
    let y_min = min.y;
    let x_max = max.x;
    let y_max = max.y;
    let min_wh = w.min(h);
    let mut tl = corners.top_left.abs().min(min_wh);
    let mut tr = corners.top_right.abs().min(min_wh);
    let mut bl = corners.bottom_left.abs().min(min_wh);
    let mut br = corners.bottom_right.abs().min(min_wh);

    // clamp border radii if they don't fit in the rectangle.
    if tl + tr > w {
        let x = (tl + tr - w) * 0.5;
        tl -= x;
        tr -= x;
    }
    if bl + br > w {
        let x = (bl + br - w) * 0.5;
        bl -= x;
        br -= x;
    }
    if tr + br > h {
        let x = (tr + br - h) * 0.5;
        tr -= x;
        br -= x;
    }
    if tl + bl > h {
        let x = (tl + bl - h) * 0.5;
        tl -= x;
        bl -= x;
    }

    // https://spencermortensen.com/articles/bezier-circle/
    const CONSTANT_FACTOR: f32 = 0.55191505;

    let tl_d = tl * CONSTANT_FACTOR;
    let tl_corner = vec2(x_min, y_min);

    let tr_d = tr * CONSTANT_FACTOR;
    let tr_corner = vec2(x_max, y_min);

    let br_d = br * CONSTANT_FACTOR;
    let br_corner = vec2(x_max, y_max);

    let bl_d = bl * CONSTANT_FACTOR;
    let bl_corner = vec2(x_min, y_max);

    let points = [
        vec2(x_min, y_min + tl),          // begin
        tl_corner + vec2(0.0, tl - tl_d), // control
        tl_corner + vec2(tl - tl_d, 0.0), // control
        tl_corner + vec2(tl, 0.0),        // end
        vec2(x_max - tr, y_min),
        tr_corner + vec2(-tr + tr_d, 0.0),
        tr_corner + vec2(0.0, tr - tr_d),
        tr_corner + vec2(0.0, tr),
        vec2(x_max, y_max - br),
        br_corner + vec2(0.0, -br + br_d),
        br_corner + vec2(-br + br_d, 0.0),
        br_corner + vec2(-br, 0.0),
        vec2(x_min + bl, y_max),
        bl_corner + vec2(bl - bl_d, 0.0),
        bl_corner + vec2(0.0, -bl + bl_d),
        bl_corner + vec2(0.0, -bl),
    ];

    builder.begin(points[0]);
    if tl > 0.0 {
        builder.cubic_bezier_to(points[1], points[2], points[3]);
    }
    builder.line_to(points[4]);

    if tl > 0.0 {
        builder.cubic_bezier_to(points[5], points[6], points[7]);
    }

    builder.line_to(points[8]);
    if br > 0.0 {
        builder.cubic_bezier_to(points[9], points[10], points[11]);
    }
    builder.line_to(points[12]);
    if bl > 0.0 {
        builder.cubic_bezier_to(points[13], points[14], points[15]);
    }
    builder.end(true);
}
