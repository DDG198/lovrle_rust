use crate::road::{rectangle_occupation, Road};
use std::cmp::min;

use anyhow::{anyhow, Result};
use rand::distributions::Bernoulli;

use crate::road::{Coord, RoadOccupier};

#[derive(Copy, Clone, Debug)]
pub struct Car {
    front: isize,
    length: usize,
    const_width: f32,
    speed: isize,
    acceleration: isize,
    speed_max: isize,
    alpha: f32,
    deceleration_distribution: Bernoulli,
}

impl RoadOccupier for Car {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        let width = self.lateral_occupancy();
        return rectangle_occupation(self.front, width as isize, width, self.length);
    }
}

impl Car {
    // fn new(
    //     front: isize,
    //     length: usize,
    //     natural_width: f32,
    //     speed: isize,
    //     acceleration: isize,
    //     max_speed: usize,
    //     alpha: f32,
    //     beta: f32,
    //     deceleration_prob: f32,
    // ) -> Self {
    //     Self {
    //         front,
    //         length: length as isize,
    //         const_width: natural_width + beta,
    //         speed,
    //         acceleration,
    //         max_speed,
    //         alpha,
    //         deceleration_prob,
    //     }
    // }

    fn lateral_occupancy(&self) -> usize {
        let additional_width = self.alpha * self.speed as f32;
        return (self.const_width + additional_width).ceil() as usize;
    }

    // pub fn speed(&self) -> isize {
    //     return self.speed;
    // }

    pub fn next_iteration_potential_speed(&self) -> isize {
        return min(self.speed + self.acceleration, self.speed_max as isize);
    }

    pub fn front(&self) -> isize {
        return self.front;
    }

    pub(crate) fn update<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
    ) -> Car {
        todo!()
    }
}

fn lateral_occupancy(width: f32, speed: isize, alpha: f32, beta: f32) -> isize {
    let additional_width = alpha * speed as f32;
    return (width + beta + additional_width).ceil() as isize;
}

struct CarBuilder {
    front: isize,
    length: usize,
    car_width: f32,
    alpha: f32,
    beta: f32,
    speed_max: isize,
    speed: isize,
    acceleration: isize,
    deceleration_prob: f64,
}

impl TryFrom<&CarBuilder> for Car {
    type Error = anyhow::Error;

    fn try_from(value: &CarBuilder) -> std::result::Result<Self, Self::Error> {
        return match value.speed_max < value.speed {
            true => Err(anyhow!(
                "speed ({}) cannot be greater than max ({})",
                value.speed_max,
                value.speed
            )),
            false => Ok(Self {
                front: value.front,
                length: value.length,
                const_width: value.car_width + value.beta,
                speed_max: value.speed_max,
                speed: value.speed,
                acceleration: value.acceleration,
                alpha: value.alpha,
                deceleration_distribution: Bernoulli::new(value.deceleration_prob)?,
            }),
        };
    }
}

impl TryFrom<CarBuilder> for Car {
    type Error = anyhow::Error;

    fn try_from(value: CarBuilder) -> Result<Self> {
        return Self::try_from(&value);
    }
}

#[cfg(test)]
mod tests {}
