use core::f32;

use skie_math::Zero;

use super::{builder::PathBuilder, Point};

#[derive(Debug, Clone, Copy, Hash, PartialEq)]
pub(crate) enum PathVerb {
    Begin,
    LineTo,
    QuadraticTo,
    CubicTo,
    Close,
    End,
}

// TODO:
// Iterator over the path to build actual points;

/// Struct to build paths
#[derive(Clone)]
pub struct Path {
    pub(crate) points: Box<[Point]>,
    pub(crate) verbs: Box<[PathVerb]>,
}

impl Path {
    pub fn create() -> DefaultPathBuilder {
        DefaultPathBuilder::default()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = PathEvent;

    type IntoIter = PathIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PathIter::new(&self.points, &self.verbs)
    }
}

#[derive(Default)]
pub struct DefaultPathBuilder {
    pub(crate) points: Vec<Point>,
    pub(crate) verbs: Vec<PathVerb>,
    first: Point,
}

impl DefaultPathBuilder {
    #[allow(unused)]
    pub fn with_capacity(points: usize, edges: usize) -> Self {
        Self {
            points: Vec::with_capacity(points),
            verbs: Vec::with_capacity(edges),
            first: Default::default(),
        }
    }

    pub fn build(self) -> Path {
        Path {
            points: self.points.into_boxed_slice(),
            verbs: self.verbs.into_boxed_slice(),
        }
    }
}

impl PathBuilder for DefaultPathBuilder {
    fn begin(&mut self, to: Point) {
        check_is_nan(to);
        self.first = to;
        self.points.push(to);
        self.verbs.push(PathVerb::Begin)
    }

    fn line_to(&mut self, to: Point) {
        check_is_nan(to);
        self.first = to;
        self.points.push(to);
        self.verbs.push(PathVerb::LineTo)
    }

    fn quadratic_bezier_to(&mut self, ctrl: Point, to: Point) {
        check_is_nan(ctrl);
        check_is_nan(to);
        self.points.push(ctrl);
        self.points.push(to);
        self.verbs.push(PathVerb::QuadraticTo);
    }

    fn cubic_bezier_to(&mut self, ctrl1: Point, ctrl2: Point, to: Point) {
        check_is_nan(ctrl1);
        check_is_nan(ctrl2);
        check_is_nan(to);
        self.points.push(ctrl1);
        self.points.push(ctrl2);
        self.points.push(to);
        self.verbs.push(PathVerb::CubicTo);
    }

    fn end(&mut self, close: bool) {
        if close {
            self.points.push(self.first);
        }

        self.verbs.push(if close {
            PathVerb::Close
        } else {
            PathVerb::End
        });
    }

    fn reserve(&mut self, endpoints: usize, ctrl_points: usize) {
        self.points.reserve(endpoints + ctrl_points);
        self.verbs.reserve(endpoints);
    }
}

#[inline]
fn check_is_nan(p: Point) {
    debug_assert!(p.x.is_finite());
    debug_assert!(p.y.is_finite());
}

pub struct PathIter<'a> {
    points: std::slice::Iter<'a, Point>,
    verbs: std::slice::Iter<'a, PathVerb>,
    first: Point,
    current: Point,
}

impl<'a> PathIter<'a> {
    pub(crate) fn new(points: &'a [Point], verbs: &'a [PathVerb]) -> Self {
        Self {
            points: points.iter(),
            verbs: verbs.iter(),
            current: Point::zero(),
            first: Point::zero(),
        }
    }

    pub fn next_point(&mut self) -> Point {
        self.points
            .next()
            .copied()
            .unwrap_or(Point::new(f32::NAN, f32::NAN))
    }
}

impl<'a> Iterator for PathIter<'a> {
    type Item = PathEvent;

    fn next(&mut self) -> Option<Self::Item> {
        match self.verbs.next() {
            Some(&PathVerb::Begin) => {
                self.current = self.next_point();
                self.first = self.current;
                Some(PathEvent::Begin { at: self.current })
            }
            Some(&PathVerb::LineTo) => {
                let from = self.current;
                self.current = self.next_point();

                Some(PathEvent::Line {
                    from,
                    to: self.current,
                })
            }
            Some(&PathVerb::QuadraticTo) => {
                let from = self.current;
                let ctrl = self.next_point();
                self.current = self.next_point();
                Some(PathEvent::Quadratic {
                    from,
                    ctrl,
                    to: self.current,
                })
            }
            Some(&PathVerb::CubicTo) => {
                let from = self.current;
                let ctrl1 = self.next_point();
                let ctrl2 = self.next_point();
                self.current = self.next_point();

                Some(PathEvent::Cubic {
                    from,
                    ctrl1,
                    ctrl2,
                    to: self.current,
                })
            }
            Some(&PathVerb::Close) => {
                let last = self.current;
                self.current = self.next_point();
                Some(PathEvent::End {
                    last,
                    first: self.first,
                    close: true,
                })
            }
            Some(&PathVerb::End) => {
                let last = self.current;
                self.current = self.first;
                Some(PathEvent::End {
                    last,
                    first: self.first,
                    close: false,
                })
            }
            None => None,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PathEvent {
    Begin {
        at: Point,
    },

    Line {
        from: Point,
        to: Point,
    },

    Quadratic {
        from: Point,
        ctrl: Point,
        to: Point,
    },

    Cubic {
        from: Point,
        ctrl1: Point,
        ctrl2: Point,
        to: Point,
    },

    End {
        last: Point,
        close: bool,
        first: Point,
    },
}

#[cfg(test)]
pub mod tests {}
