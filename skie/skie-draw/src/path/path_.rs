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

/// Struct to store built paths
#[derive(Clone, Debug)]
pub struct Path {
    pub(crate) points: Box<[Point]>,
    pub(crate) verbs: Box<[PathVerb]>,
}

impl Path {
    pub fn builder() -> PathBuilder {
        PathBuilder::default()
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = PathEvent;

    type IntoIter = PathIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PathIter::new(&self.points, &self.verbs)
    }
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
