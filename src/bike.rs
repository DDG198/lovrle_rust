use std::iter::{repeat, zip};

use crate::road::RoadOccupier;

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

impl RoadOccupier for Bike {
    fn occupied_cells(&self) -> impl IntoIterator<Item = (isize, isize)> {
        return (self.right..(self.right + self.width))
            .map(|x| zip(repeat(x), (self.front - self.length)..(self.front)))
            .flatten();
    }
}
