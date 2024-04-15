use crate::road::{rectangle_occupation, Road, Vehicle};
use std::cmp::{max, min};

use anyhow::{anyhow, Result};
use rand::{distributions::Bernoulli, prelude::Distribution};
use serde::Serialize;

use crate::road::{Coord, RoadOccupier};

#[derive(Copy, Clone, Debug)]
pub struct Car {
    front: isize,
    length: usize,
    const_width: f32,
    pub speed: isize,
    fast_acceleration: isize,
    slow_acceleration: isize,
    max_slow_speed: isize,
    speed_max: isize,
    alpha: f32,
    deceleration_distribution: Bernoulli,
}

impl RoadOccupier for Car {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        let width = self.lateral_occupancy();
        return rectangle_occupation(self.front, (width as isize) - 1, width, self.length);
    }
}

impl Car {
    // fn lateral_occupancy(&self) -> usize {
    //     let additional_width = self.alpha * self.speed as f32;
    //     return (self.const_width + additional_width).ceil() as usize;
    // }

    pub fn next_iteration_potential_speed(&self) -> isize {
        let acceleration = match self.speed <= self.max_slow_speed {
            true => self.slow_acceleration,
            false => self.fast_acceleration,
        };
        return min(self.speed + acceleration, self.speed_max as isize);
    }

    pub const fn front(&self) -> isize {
        return self.front;
    }

    pub fn safe_speeds<
        'a,
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &'a self,
        road: &'a Road<B, C, L, BLW, MLW>,
        self_id: usize,
    ) -> impl Iterator<Item = isize> + 'a {
        return (0..=self.next_iteration_potential_speed()).filter(move |speed| {
            let potential_car = Self {
                front: self.front + speed,
                speed: *speed,
                ..*self
            };

            !road.is_collision_for(&potential_car, Vehicle::Car(self_id))
        });
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
        self_id: usize,
    ) -> Self {
        // this implementation is different from that described in the paper as
        // the paper implementation caused collisions between vehicles.

        // ..= as if your max_speed is 1 you'll want to be able to go 1 ahead.
        debug_assert_ne!(self.next_iteration_potential_speed(), 0);
        let mut next_speed = self.fastest_safe_speed(road, self_id);

        // cannot cause issues with the previous speed being unsafe as
        next_speed = match self.should_decelerate() {
            true => max(next_speed - 1, 0),
            false => next_speed,
        };

        return Car {
            front: (self.front + next_speed).rem_euclid(L as isize),
            speed: next_speed,
            ..*self
        };
    }

    fn should_decelerate(&self) -> bool {
        return self
            .deceleration_distribution
            .sample(&mut rand::thread_rng());
    }

    fn lateral_occupancy_at_speed(&self, speed: isize) -> usize {
        return lateral_occupancy(self.const_width, speed, self.alpha);
    }

    fn lateral_occupancy(&self) -> usize {
        return self.lateral_occupancy_at_speed(self.speed);
    }

    fn fastest_safe_speed<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
        self_id: usize,
    ) -> isize {
        (1..=self.next_iteration_potential_speed())
            .take_while(|speed| {
                let potential_car = Self {
                    front: self.front + speed,
                    speed: *speed,
                    ..*self
                };
                !road.is_collision_for(&potential_car, Vehicle::Car(self_id))
            })
            .last()
            .unwrap_or(0)
    }
}

fn lateral_occupancy(const_width: f32, speed: isize, alpha: f32) -> usize {
    let additional_width = alpha * speed as f32;
    return (const_width + additional_width).ceil() as usize;
}

#[derive(Debug, Serialize, Copy, Clone)]
pub struct CarBuilder {
    front: isize,
    length: usize,
    car_width: f32,
    alpha: f32,
    beta: f32,
    speed_max: isize,
    speed: isize,
    deceleration_prob: f64,
    slow_acceleration: isize,
    fast_acceleration: isize,
    max_slow_speed: isize,
}

impl CarBuilder {
    pub fn with_front_at(&self, front: isize) -> Self {
        return Self { front, ..*self };
    }

    pub fn with_slow_acceleration(&self, slow_acceleration: isize) -> Self {
        return Self {
            slow_acceleration,
            ..*self
        };
    }

    pub fn build(&self) -> Result<Car> {
        return self.try_into();
    }

    fn with_speed(&self, speed: isize) -> Self {
        return Self { speed, ..*self };
    }

    fn with_deceleration_prob(&self, deceleration_prob: f64) -> Result<Self> {
        return match deceleration_prob <= 0.0 && 1.0 <= deceleration_prob {
            true => Err(anyhow!(
                "deceleration_prob must be between 0 and 1, instead {}",
                deceleration_prob
            )),
            false => Ok(Self {
                deceleration_prob,
                ..*self
            }),
        };
    }
}

impl Default for CarBuilder {
    fn default() -> Self {
        Self {
            front: 5,
            length: 5,
            car_width: 3.6,
            alpha: 0.26,
            beta: 0.6,
            speed_max: 20,
            speed: 0,
            slow_acceleration: 2,
            fast_acceleration: 1,
            max_slow_speed: 5,
            deceleration_prob: 0.2,
        }
    }
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
                fast_acceleration: value.fast_acceleration,
                slow_acceleration: value.slow_acceleration,
                max_slow_speed: value.max_slow_speed,
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
mod tests {
    use crate::road::Road;

    use crate::car::CarBuilder;

    #[test]
    fn car_update_works() {
        let cars = [CarBuilder::default()].map(|builder| builder.try_into().unwrap());
        let mut road = Road::<0, 1, 20, 3, 3>::new([], cars).unwrap();

        road.cars_update().unwrap();
    }

    #[test]
    fn car_update_works_as_expected() {
        let start_front = 10;
        let slow_acc = 2;
        let cars = [CarBuilder::default()
            .with_front_at(start_front)
            .with_slow_acceleration(slow_acc)
            .with_speed(0)
            .with_deceleration_prob(0.0)
            .unwrap()]
        .map(|builder| builder.try_into().unwrap());
        let mut road = Road::<0, 1, 20, 3, 3>::new([], cars).unwrap();

        road.cars_update().unwrap();

        let end_front = road.get_car(0).front;

        assert_eq!(end_front - start_front, slow_acc);
    }
}
