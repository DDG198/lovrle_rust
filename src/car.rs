use std::{
    cmp::min,
    iter::{repeat, zip},
};

use crate::road::{Coord, RoadOccupier};

pub struct Car {
    front: isize,
    length: isize,
    const_width: f32,
    speed: isize,
    acceleration: isize,
    max_speed: usize,
    alpha: f32,
    deceleration_prob: f32,
}

impl RoadOccupier for Car {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        return (0isize..self.lateral_occupancy())
            .map(|lat| zip(repeat(lat), (self.front - self.length)..(self.front)))
            .flatten()
            .map(|(lat, long)| Coord { lat, long });
    }
}

impl Car {
    fn new(
        front: isize,
        length: usize,
        natural_width: f32,
        speed: isize,
        acceleration: isize,
        max_speed: usize,
        alpha: f32,
        beta: f32,
        deceleration_prob: f32,
    ) -> Self {
        Self {
            front,
            length: length as isize,
            const_width: natural_width + beta,
            speed,
            acceleration,
            max_speed,
            alpha,
            deceleration_prob,
        }
    }

    fn lateral_occupancy(&self) -> isize {
        let additional_width = self.alpha * self.speed as f32;
        return (self.const_width + additional_width).ceil() as isize;
    }

    // pub fn speed(&self) -> isize {
    //     return self.speed;
    // }

    pub fn next_iteration_potential_speed(&self) -> isize {
        return min(self.speed + self.acceleration, self.max_speed as isize);
    }

    pub fn front(&self) -> isize {
        return self.front;
    }
}
