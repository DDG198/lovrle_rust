use std::iter::{repeat, zip};

use crate::road::RoadOccupier;

#[derive(Debug)]
pub struct Bike {
    pub front: isize,
    pub right: isize,
    pub length: isize,
    pub width: isize,
    forward_speed_max: isize,
    forward_speed: isize,
    forward_acceleration: isize,
    rightward_speed_max: isize,
    rightward_speed: isize,
}

impl Bike {
    /// Returns the positions that the bike could move to laterally
    pub fn potential_lateral_positions(&self) -> impl IntoIterator<Item = isize> {
        return self.right - self.rightward_speed_max..self.right + self.rightward_speed_max;
    }
}

impl RoadOccupier for Bike {
    fn occupied_cells(&self) -> impl IntoIterator<Item = (isize, isize)> {
        return (self.right..(self.right + self.width))
            .map(|x| zip(repeat(x), (self.front - self.length)..(self.front)))
            .flatten();
    }
}
