use std::ops::Range;

use skie_math::Rect;

use crate::paint::{CubicBezier, QuadraticBezier};

use super::{Contour, PathEvent, Point};

pub struct PathGeometryBuilder<'a, PathIter>
where
    PathIter: Iterator<Item = PathEvent>,
{
    output: &'a mut Vec<Point>,
    offset: usize,
    num_segments: u32,
    path_iter: PathIter,
}

impl<'a, PathIter> PathGeometryBuilder<'a, PathIter>
where
    PathIter: Iterator<Item = PathEvent>,
{
    pub fn new(path_iter: impl Into<PathIter>, output: &'a mut Vec<Point>, clear: bool) -> Self {
        if clear {
            output.clear()
        }
        let offset = output.len();

        Self {
            output,
            offset,
            num_segments: 16,
            path_iter: path_iter.into(),
        }
    }

    fn build_geometry_till_end(&mut self, start: Point) -> Contour {
        self.output.push(start);

        loop {
            match self.path_iter.next() {
                Some(PathEvent::Begin { .. }) => unreachable!("invalid geometry"),
                Some(PathEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to,
                }) => {
                    // todo in case of 0 num_segments;
                    let bezier = CubicBezier {
                        from,
                        ctrl1,
                        ctrl2,
                        to,
                    };

                    let num_segments = self.num_segments;
                    let t_step = 1.0 / num_segments as f32;
                    self.output.reserve(num_segments as usize);

                    for i in 1..=num_segments {
                        self.output.push(bezier.sample(t_step * i as f32))
                    }
                }
                Some(PathEvent::Quadratic { from, ctrl, to }) => {
                    // todo in case of 0 num_segments;
                    let bezier = QuadraticBezier { from, ctrl, to };
                    let num_segments = self.num_segments;
                    let t_step = 1.0 / num_segments as f32;
                    self.output.reserve(num_segments as usize);

                    for i in 1..=num_segments {
                        self.output.push(bezier.sample(t_step * i as f32))
                    }
                }
                Some(PathEvent::Line { to, .. }) => self.output.push(to),
                Some(PathEvent::End {
                    close,
                    first,
                    contour,
                    ..
                }) => {
                    if close {
                        self.output.push(first)
                    }
                    return contour;
                }
                None => return Contour::INVALID,
            }
        }
    }
}

impl<'a, PathIter> Iterator for PathGeometryBuilder<'a, PathIter>
where
    PathIter: Iterator<Item = PathEvent>,
{
    type Item = (Contour, Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.path_iter.next() {
            Some(PathEvent::Begin { at }) => {
                let start = self.offset;
                let contour = self.build_geometry_till_end(at);
                let end = self.output.len();
                self.offset = end;
                Some((contour, start..end))
            }

            None => None,
            _ => {
                // this should not happen
                unreachable!("invalid path")
            }
        }
    }
}

pub fn get_path_bounds(path: &[Point]) -> Rect<f32> {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;

    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for point in path {
        let x = point.x;
        let y = point.y;
        min_x = if x < min_x { x } else { min_x };
        max_x = if x > max_x { x } else { max_x };

        min_y = if y < min_y { y } else { min_y };
        max_y = if y > max_y { y } else { max_y };
    }

    Rect::from_corners((min_x, min_y).into(), (max_x, max_y).into())
}

#[cfg(test)]
mod tests {
    use crate::path::{PathBuilder, PathEventsIter, Point};
    use skie_math::{vec2, Corners, Rect};

    use super::PathGeometryBuilder;

    #[test]
    fn path_geometry_build_basic() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();
        path.begin(vec2(0.0, 0.0));
        path.line_to(vec2(0.0, 10.0));
        path.line_to(vec2(0.0, 20.0));
        path.line_to(vec2(0.0, 30.0));
        path.end(false);

        path.begin(vec2(100.0, 100.0));
        path.line_to(vec2(200.0, 300.0));
        path.close();

        let geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false);

        let contours = geo_build.map(|v| v.1).collect::<Vec<_>>();

        assert_eq!(output.len(), 7);
        assert_eq!(contours.len(), 2);

        {
            let start = contours[0].start;
            let end = contours[0].end;
            let points = &output[start..end];

            assert_eq!(
                points,
                &[
                    vec2(0.0, 0.0),
                    vec2(0.0, 10.0),
                    vec2(0.0, 20.0),
                    vec2(0.0, 30.0),
                ]
            );
        }

        {
            let start = contours[1].start;
            let end = contours[1].end;
            let points = &output[start..end];
            assert_eq!(
                points,
                &[vec2(100.0, 100.0), vec2(200.0, 300.0), vec2(100.0, 100.0),]
            );
        }
    }

    #[test]
    fn path_geometry_contours() {
        let mut output = <Vec<Point>>::new();
        let mut path = PathBuilder::default();
        path.begin(vec2(0.0, 0.0));
        path.line_to(vec2(0.0, 10.0));
        path.line_to(vec2(0.0, 20.0));
        path.line_to(vec2(0.0, 30.0));
        path.end(false);

        path.begin(vec2(100.0, 100.0));
        path.line_to(vec2(200.0, 300.0));
        path.close();

        path.circle(vec2(0.0, 0.0), 5.0);
        path.rect(&Rect::xywh(10.0, 10.0, 100.0, 100.0));
        path.round_rect(
            &Rect::xywh(100.0, 100.0, 100.0, 100.0),
            &Corners::with_all(20.0),
        );

        let geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false);

        let contours = geo_build.collect::<Vec<_>>();
        assert_eq!(contours.len(), 5);
    }

    #[test]
    fn path_geometry_quadratic_bezier() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();
        path.begin(vec2(0.0, 0.0));
        path.quadratic_to(vec2(5.0, 5.0), vec2(10.0, 0.0));
        path.end(false);

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false)
                .map(|v| v.1);
        let range = geo_build.next().expect("no countours found");
        assert!(geo_build.next().is_none());

        let points = &output[range];
        assert_eq!(
            points,
            &[
                vec2(0.0, 0.0),
                vec2(0.625, 0.5859375),
                vec2(1.25, 1.09375),
                vec2(1.875, 1.5234375),
                vec2(2.5, 1.875),
                vec2(3.125, 2.1484375),
                vec2(3.75, 2.34375),
                vec2(4.375, 2.4609375),
                vec2(5.0, 2.5),
                vec2(5.625, 2.4609375),
                vec2(6.25, 2.34375),
                vec2(6.875, 2.1484375),
                vec2(7.5, 1.875),
                vec2(8.125, 1.5234375),
                vec2(8.75, 1.09375),
                vec2(9.375, 0.5859375),
                vec2(10.0, 0.0),
            ]
        );
    }

    #[test]
    fn path_geometry_cubic_bezier() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();
        path.begin(vec2(0.0, 0.0));
        path.cubic_to(vec2(0.0, 4.0), vec2(6.0, 0.0), vec2(6.0, 8.0));
        path.end(false);

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false)
                .map(|v| v.1);

        let range = geo_build.next().expect("no contours found");
        let points = &output[range];

        let expected_points = [
            vec2(0.0, 0.0),
            vec2(0.06738281, 0.6611328),
            vec2(0.2578125, 1.1640625),
            vec2(0.55371094, 1.5380859),
            vec2(0.9375, 1.8125),
            vec2(1.3916016, 2.0166016),
            vec2(1.8984375, 2.1796875),
            vec2(2.4404297, 2.3310547),
            vec2(3.0, 2.5),
            vec2(3.5595703, 2.7158203),
            vec2(4.1015625, 3.0078125),
            vec2(4.6083984, 3.4052734),
            vec2(5.0625, 3.9375),
            vec2(5.446289, 4.633789),
            vec2(5.7421875, 5.5234375),
            vec2(5.932617, 6.635742),
            vec2(6.0, 8.),
        ];

        assert_eq!(points, &expected_points);
    }

    #[test]
    fn path_geometry_circle() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();
        path.circle(vec2(0.0, 0.0), 5.0);

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false)
                .map(|v| v.1);

        let range = geo_build.next().expect("no contours found");
        let points = &output[range];

        assert_eq!(
            points,
            &[
                vec2(-5.0, 0.0),
                vec2(-4.9741654, -0.51091635),
                vec2(-4.898342, -1.0071437),
                vec2(-4.7750516, -1.4861608),
                vec2(-4.6068153, -1.9454459),
                vec2(-4.396155, -2.3824778),
                vec2(-4.1455913, -2.7947354),
                vec2(-3.857646, -3.1796968),
                vec2(-3.5348408, -3.5348408),
                vec2(-3.1796968, -3.857646),
                vec2(-2.7947354, -4.1455913),
                vec2(-2.3824778, -4.396155),
                vec2(-1.9454459, -4.6068153),
                vec2(-1.4861608, -4.7750516),
                vec2(-1.0071437, -4.898342),
                vec2(-0.51091635, -4.9741654),
                vec2(0.0, -5.0),
                vec2(0.51091635, -4.9741654),
                vec2(1.0071437, -4.898342),
                vec2(1.4861608, -4.7750516),
                vec2(1.9454459, -4.6068153),
                vec2(2.3824778, -4.396155),
                vec2(2.7947354, -4.1455913),
                vec2(3.1796968, -3.857646),
                vec2(3.5348408, -3.5348408),
                vec2(3.857646, -3.1796968),
                vec2(4.1455913, -2.7947354),
                vec2(4.396155, -2.3824778),
                vec2(4.6068153, -1.9454459),
                vec2(4.7750516, -1.4861608),
                vec2(4.898342, -1.0071437),
                vec2(4.9741654, -0.51091635),
                vec2(5.0, 0.0),
                vec2(4.9741654, 0.51091635),
                vec2(4.898342, 1.0071437),
                vec2(4.7750516, 1.4861608),
                vec2(4.6068153, 1.9454459),
                vec2(4.396155, 2.3824778),
                vec2(4.1455913, 2.7947354),
                vec2(3.857646, 3.1796968),
                vec2(3.5348408, 3.5348408),
                vec2(3.1796968, 3.857646),
                vec2(2.7947354, 4.1455913),
                vec2(2.3824778, 4.396155),
                vec2(1.9454459, 4.6068153),
                vec2(1.4861608, 4.7750516),
                vec2(1.0071437, 4.898342),
                vec2(0.51091635, 4.9741654),
                vec2(0.0, 5.0),
                vec2(-0.51091635, 4.9741654),
                vec2(-1.0071437, 4.898342),
                vec2(-1.4861608, 4.7750516),
                vec2(-1.9454459, 4.6068153),
                vec2(-2.3824778, 4.396155),
                vec2(-2.7947354, 4.1455913),
                vec2(-3.1796968, 3.857646),
                vec2(-3.5348408, 3.5348408),
                vec2(-3.857646, 3.1796968),
                vec2(-4.1455913, 2.7947354),
                vec2(-4.396155, 2.3824778),
                vec2(-4.6068153, 1.9454459),
                vec2(-4.7750516, 1.4861608),
                vec2(-4.898342, 1.0071437),
                vec2(-4.9741654, 0.51091635),
                vec2(-5.0, 0.0),
                vec2(-5.0, 0.0),
            ]
        );
    }
    #[test]
    fn path_geometry_round_rect() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();

        path.round_rect(
            &Rect::xywh(10.0, 10.0, 100.0, 100.0),
            &Corners::with_all(20.0),
        );

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false)
                .map(|v| v.1);

        let range = geo_build.next().expect("no contours found");
        let points = &output[range];

        assert_eq!(
            &points,
            &[
                vec2(10.0, 30.0),
                vec2(10.103339, 27.956335),
                vec2(10.406632, 25.971424),
                vec2(10.899794, 24.055357),
                vec2(11.572739, 22.218216),
                vec2(12.4153805, 20.470089),
                vec2(13.417635, 18.821058),
                vec2(14.569416, 17.281212),
                vec2(15.860637, 15.860637),
                vec2(17.281212, 14.569416),
                vec2(18.821058, 13.417635),
                vec2(20.470089, 12.4153805),
                vec2(22.218216, 11.572739),
                vec2(24.055357, 10.899794),
                vec2(25.971424, 10.406632),
                vec2(27.956335, 10.103339),
                vec2(30.0, 10.0),
                vec2(90.0, 10.0),
                vec2(92.04366, 10.103339),
                vec2(94.02857, 10.406632),
                vec2(95.94464, 10.899794),
                vec2(97.781784, 11.572739),
                vec2(99.52991, 12.4153805),
                vec2(101.17894, 13.417635),
                vec2(102.71878, 14.569416),
                vec2(104.13936, 15.860637),
                vec2(105.43059, 17.281212),
                vec2(106.58237, 18.821058),
                vec2(107.58462, 20.470089),
                vec2(108.42726, 22.218216),
                vec2(109.100204, 24.055357),
                vec2(109.59337, 25.971424),
                vec2(109.89666, 27.956335),
                vec2(110.0, 30.0),
                vec2(110.0, 90.0),
                vec2(109.89666, 92.04366),
                vec2(109.59337, 94.02857),
                vec2(109.100204, 95.94464),
                vec2(108.42726, 97.781784),
                vec2(107.58462, 99.52991),
                vec2(106.58237, 101.17894),
                vec2(105.43059, 102.71878),
                vec2(104.13936, 104.13936),
                vec2(102.71878, 105.43059),
                vec2(101.17894, 106.58237),
                vec2(99.52991, 107.58462),
                vec2(97.781784, 108.42726),
                vec2(95.94464, 109.100204),
                vec2(94.02857, 109.59337),
                vec2(92.04366, 109.89666),
                vec2(90.0, 110.0),
                vec2(30.0, 110.0),
                vec2(27.956335, 109.89666),
                vec2(25.971424, 109.59337),
                vec2(24.055357, 109.100204),
                vec2(22.218216, 108.42726),
                vec2(20.470089, 107.58462),
                vec2(18.821058, 106.58237),
                vec2(17.281212, 105.43059),
                vec2(15.860637, 104.13936),
                vec2(14.569416, 102.71878),
                vec2(13.417635, 101.17894),
                vec2(12.4153805, 99.52991),
                vec2(11.572739, 97.781784),
                vec2(10.899794, 95.94464),
                vec2(10.406632, 94.02857),
                vec2(10.103339, 92.04366),
                vec2(10.0, 90.0),
                vec2(10.0, 30.0),
            ]
        );
    }
}
