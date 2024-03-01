use std::{
    collections::HashMap,
    iter::{repeat, zip},
};

use crate::road::{RoadCells, RoadOccupier, Vehicle};

pub struct Bike {
    front: isize,
    right: isize,
    length: isize,
    width: isize,
    forward_speed_max: isize,
    forward_speed: isize,
    forward_acceleration: isize,
    rightward_speed_max: isize,
    rightward_speed: isize,
}
impl Bike {
    pub fn lateral_update<const L: usize, const BLW: usize, const MLW: usize>(
        &self,
        cells: &RoadCells<L, BLW, MLW>,
    ) -> Bike {
        let mut valid_ys = (-self.rightward_speed_max..self.rightward_speed_max)
            .map(|rightward_speed| self.right + rightward_speed)
            .filter(|new_right| {
                let potential_bike = Bike {
                    right: *new_right,
                    ..*self
                };
                true
            });
        Bike { ..*self }
    }
}

impl RoadOccupier for Bike {
    fn occupied_cells(&self) -> impl IntoIterator<Item = (isize, isize)> {
        return (self.right..(self.right + self.width))
            .map(|x| zip(repeat(x), (self.front - self.length)..(self.front)))
            .flatten();
    }
}
