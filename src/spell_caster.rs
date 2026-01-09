use crate::{Point, Spell};
use alloc::vec::Vec;

pub struct SpellBuilder {
    points: Vec<Point>,
    last_point: Point,
}

impl Default for SpellBuilder {
    fn default() -> Self {
        Self {
            points: Vec::with_capacity(1_000),
            last_point: (0, 0),
        }
    }
}

impl SpellBuilder {
    pub fn step(&mut self, point: Point) {
        if point != (0, 0) && point != self.last_point {
            self.points.push(point);
        }

        self.last_point = point;
    }

    pub fn should_cast(&self) -> bool {
        self.last_point == (0, 0) && !self.points.is_empty()
    }

    pub fn build(&self) -> Spell {
        self.points.clone()
    }

    pub fn reset(&mut self) {
        self.points.clear();
        self.last_point = (0, 0);
    }
}
