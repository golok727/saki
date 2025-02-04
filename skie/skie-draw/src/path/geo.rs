use std::ops::Range;

use crate::paint::QuadraticBezier;

use super::{PathEvent, Point};

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

    fn build_geometry_till_end(&mut self, start: Point) {
        self.output.push(start);

        loop {
            match self.path_iter.next() {
                Some(PathEvent::Begin { .. }) => unreachable!("invalid geometry"),
                Some(PathEvent::Cubic { .. }) => {
                    todo!("Cubic not implemented yet")
                }
                Some(PathEvent::Quadratic { from, ctrl, to }) => {
                    // todo in case of 0 num_segments;
                    let bezier = QuadraticBezier { from, ctrl, to };
                    let num_segments = self.num_segments;
                    let t_step = 1.0 / num_segments as f32;
                    self.output.reserve(num_segments as usize + 1);

                    for i in 1..=num_segments {
                        self.output.push(bezier.sample(t_step * i as f32))
                    }
                }
                Some(PathEvent::Line { to, .. }) => self.output.push(to),
                Some(PathEvent::End { close, first, .. }) => {
                    if close {
                        self.output.push(first)
                    }
                    return;
                }
                None => return,
            }
        }
    }
}

impl<'a, PathIter> Iterator for PathGeometryBuilder<'a, PathIter>
where
    PathIter: Iterator<Item = PathEvent>,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.path_iter.next() {
            Some(PathEvent::Begin { at }) => {
                let start = self.offset;
                self.build_geometry_till_end(at);
                let end = self.output.len();
                self.offset = end;
                Some(start..end)
            }

            None => None,
            _ => {
                // this should not happen
                unreachable!("invalid path")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::path::{PathBuilder, PathEventsIter, Point};
    use skie_math::vec2;

    use super::PathGeometryBuilder;

    #[test]
    fn path_geometry_quadratic_bezier() {
        let mut output = <Vec<Point>>::new();

        let mut path = PathBuilder::default();
        path.begin(vec2(0.0, 0.0));
        path.quadratic_to(vec2(5.0, 5.0), vec2(10.0, 0.0));
        path.end(false);

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false);
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
        path.end(false);

        let mut geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false);

        let range = geo_build.next().expect("no countours found");
        let points = &output[range];

        {
            use std::fmt::Write;
            let mut out = String::new();
            out.push('[');
            for point in points {
                write!(&mut out, "({}, {}),", point.x, point.y).unwrap();
            }
            out.push_str("]\n");
            println!("{out}");
        }

        // {
        //     use std::fmt::Write;
        //     let mut out = String::new();
        //     out.push_str("assert_eq!(&path.points, &[\n");
        //     for point in points {
        //         writeln!(&mut out, "vec2({:.07}, {:.07}),", point.x, point.y).unwrap();
        //     }
        //     out.push_str("]);\n");
        //     println!("{out}");
        // }
    }

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
        path.line_to(vec2(200.0, 200.0));
        path.close();

        let geo_build =
            <PathGeometryBuilder<PathEventsIter>>::new(path.path_events(), &mut output, false);

        let contours = geo_build.collect::<Vec<_>>();

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
                &[vec2(100.0, 100.0), vec2(200.0, 200.0), vec2(100.0, 100.0),]
            );
        }
    }
}
