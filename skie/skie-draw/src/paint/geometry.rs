use crate::path::Point;

pub struct QuadraticBezier {
    pub from: Point,
    pub to: Point,
    pub ctrl: Point,
}

impl QuadraticBezier {
    pub fn sample(&self, t: f32) -> Point {
        let one_minus_t = 1.0 - t;
        let p0 = self.from * (one_minus_t * one_minus_t);
        let p1 = self.ctrl * (2.0 * one_minus_t * t);
        let p2 = self.to * (t * t);

        p0 + p1 + p2
    }
}
