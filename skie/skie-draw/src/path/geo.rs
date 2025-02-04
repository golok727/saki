use std::ops::Range;

use super::{PathEvent, Point};

pub struct PathGeometryBuilder<'a, PathIter>
where
    PathIter: Iterator<Item = PathEvent>,
{
    output: &'a mut Vec<Point>,
    offset: usize,
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
                Some(PathEvent::Quadratic { .. }) => {
                    todo!("Quadratic not implemented yet")
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
