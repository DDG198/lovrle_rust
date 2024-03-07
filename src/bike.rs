use std::{
    cmp::Ordering,
    iter::{repeat, zip},
};

use anyhow::{anyhow, Result};
use rand::{distributions::Bernoulli, prelude::Distribution};

use crate::road::{RectangleOccupier, Road, RoadOccupier, Vehicle};

#[derive(Debug)]
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
    ignore_lateral_distribution: Bernoulli,
}

impl Bike {
    const fn left(&self) -> isize {
        return self.right - self.width;
    }

    const fn back(&self) -> isize {
        return self.front - self.length;
    }
    /// Returns the positions that the bike could move to laterally
    pub const fn potential_lateral_positions(&self) -> impl Iterator<Item = isize> {
        // could add something to do with the width of the bike here,
        // ensuring that the lhs of the bike is not off the road.
        // Could also put a similar check in for the right side? but
        // that would require knowledge of the road width.
        // Leave this as an optimisation for the future.
        return self.right - self.rightward_speed_max..self.right + self.rightward_speed_max;
    }

    fn should_ignore_lateral_movement(&self) -> bool {
        return self
            .ignore_lateral_distribution
            .sample(&mut rand::thread_rng());
    }

    fn y_j_t_plus_1(&self) -> impl Iterator<Item = isize> {
        return self.potential_lateral_positions();
    }

    pub fn self_lateral_update<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        self_id: usize,
        road: &Road<B, C, L, BLW, MLW>,
    ) -> Self {
        if self.should_ignore_lateral_movement() {
            return Self { ..*self };
        }
        // Y'_{j,t+1}
        let y_prime_j_t_plus_1 = self.y_prime_j_t_plus_1(road, &self_id);

        let current_occupation = self.rectangle_occupation();
        let y_prime_prime_j_t_plus_1: Vec<RectangleOccupier> =
            match road.motor_lane_contains_occupier(&current_occupation) {
                // on motor lane
                true => match road.is_blocking(&current_occupation.back_left(), None) {
                    // motor lane blocking
                    true => {
                        let mut on_motor_lane = Vec::<RectangleOccupier>::new();
                        let mut on_bike_lane = Vec::<RectangleOccupier>::new();

                        for occupier in y_prime_j_t_plus_1 {
                            match road.motor_lane_contains_occupier(&occupier) {
                                true => on_motor_lane.push(occupier),
                                false => on_bike_lane.push(occupier),
                            }
                        }

                        // if can move to bike lane:
                        //   - bike lane occupations
                        // else
                        //   - furthest right occupation
                        match on_bike_lane.is_empty() {
                            true => vec![*on_motor_lane
                                .last() // assuming that y_prime is left to right
                                .expect("bike should be able to stay still")],
                            false => on_bike_lane,
                        }
                    }
                    // motor lane non-blocking
                    // check exactly what the boundary should be here: lhs or rhs
                    false => Self::avoid_blocking_ypp_filter(
                        // into_iter here and below for debugging
                        y_prime_j_t_plus_1,
                        &road,
                        current_occupation.right,
                    )
                    .collect(),
                },
                // on bike lane
                false => Self::avoid_blocking_ypp_filter(y_prime_j_t_plus_1, &road, MLW as isize)
                    .collect(),
            };

        // select Y'' with the furthest front gap
        debug_assert!(!y_prime_prime_j_t_plus_1.is_empty());
        let selected_occupation = select_y_star(y_prime_prime_j_t_plus_1, road);
        Self {
            right: selected_occupation.right,
            ..*self
        }
    }

    fn y_prime_j_t_plus_1<
        'a,
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &'a self,
        road: &'a Road<B, C, L, BLW, MLW>,
        self_id: &'a usize,
    ) -> impl Iterator<Item = RectangleOccupier> + '_ {
        return self
            .y_j_t_plus_1()
            // Step 1: check the availability of possible lateral positions
            .map(|position| RectangleOccupier {
                front: self.front,
                right: position,
                width: self.width,
                length: self.length,
            })
            // check that the occupation is on the road
            .filter(|occupation| road.road_contains_occupier(occupation))
            .filter(|occupation| !road.is_collision_for(occupation, Vehicle::Bike(*self_id)));
        // .filter(|potential_occupation| {
        //     road.collisions_for(potential_occupation)
        //         .into_iter()
        //         // only a collision if the found vehicle is not this bike
        //         .any(|found_vehicle| match found_vehicle {
        //             Vehicle::Bike(found_bike_id) => *found_bike_id == *self_id,
        //             Vehicle::Car(_) => false,
        //         })
        // });
    }

    fn y_star_cmp_priority<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        road: &Road<B, C, L, BLW, MLW>,
        lhs: &RectangleOccupier,
        rhs: &RectangleOccupier,
    ) -> Ordering {
        match road.front_gap(lhs).cmp(&road.front_gap(rhs)) {
            Ordering::Less => Ordering::Less,
            Ordering::Equal => match (
                road.motor_lane_contains_occupier(lhs),
                road.motor_lane_contains_occupier(rhs),
            ) {
                (true, true) => lhs.left().cmp(&rhs.left()),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => Ordering::Equal,
            },
            Ordering::Greater => Ordering::Greater,
        }
    }

    fn avoid_blocking_ypp_filter<
        'a,
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        yp: impl Iterator<Item = RectangleOccupier> + 'a,
        road: &'a Road<B, C, L, BLW, MLW>,
        boundary: isize,
    ) -> impl Iterator<Item = RectangleOccupier> + '_ {
        yp.filter(
            move |occupation| match occupation.occupier_is_within(boundary) {
                true => road.is_blocking(&occupation.back_left(), None),
                false => true,
            },
        )
    }

    pub const fn rectangle_occupation(&self) -> RectangleOccupier {
        return RectangleOccupier {
            front: self.front,
            right: self.right,
            width: self.width,
            length: self.length,
        };
    }
}

fn select_y_star<
    const B: usize,
    const C: usize,
    const L: usize,
    const BLW: usize,
    const MLW: usize,
>(
    mut choices: Vec<RectangleOccupier>,
    road: &Road<B, C, L, BLW, MLW>,
) -> RectangleOccupier {
    let mut best_yet = vec![choices.pop().expect("bike should be able to stay still")];

    for potential in choices {
        match Bike::y_star_cmp_priority(
            road,
            &potential,
            best_yet.first().expect("this vector should never be empty"),
        ) {
            Ordering::Less => (),
            Ordering::Equal => best_yet.push(potential),
            Ordering::Greater => {
                best_yet.clear();
                best_yet.push(potential);
            }
        }
    }

    return *best_yet.first().unwrap();
}

impl RoadOccupier for Bike {
    fn occupied_cells(&self) -> impl Iterator<Item = (isize, isize)> {
        return (self.right..(self.right + self.width))
            .map(|x| zip(repeat(x), (self.front - self.length)..(self.front)))
            .flatten();
    }
}

pub struct BikeBuilder {
    front: isize,
    right: isize,
    length: isize,
    width: isize,
    forward_speed_max: isize,
    forward_speed: isize,
    forward_acceleration: isize,
    rightward_speed_max: isize,
    lateral_ignorance: f64,
}

impl BikeBuilder {
    pub const fn with_front_at(&self, front: isize) -> Self {
        return Self { front, ..*self };
    }

    pub const fn with_right_at(&self, right: isize) -> Self {
        return Self { right, ..*self };
    }

    pub const fn with_front_right_at(&self, front_right: (isize, isize)) -> Self {
        let (front, right) = front_right;
        return self.with_front_at(front).with_right_at(right);
    }

    pub fn with_length(&self, length: isize) -> Result<Self> {
        return match length < 1 {
            true => Err(anyhow!(
                "length must be strictly positive, instead {}",
                length
            )),
            false => Ok(Self { length, ..*self }),
        };
    }

    pub fn with_width(&self, width: isize) -> Result<Self> {
        return match width < 1 {
            true => Err(anyhow!(
                "width must be strictly positive, instead {}",
                width
            )),
            false => Ok(Self { width, ..*self }),
        };
    }

    pub fn with_dimensions(&self, dimensions: (isize, isize)) -> Result<Self> {
        let (width, length) = dimensions;
        return Ok(self.with_width(width)?.with_length(length)?);
    }

    pub fn with_forward_max_speed(&self, forward_speed_max: isize) -> Result<Self> {
        return match forward_speed_max.is_negative() {
            true => Err(anyhow!(
                "cannot have negative max speed, instead {}",
                forward_speed_max
            )),
            false => Ok(Self {
                forward_speed_max,
                ..*self
            }),
        };
    }

    pub fn with_forward_speed(&self, forward_speed: isize) -> Result<Self> {
        return match forward_speed.is_negative() {
            true => Err(anyhow!(
                "cannot have negative speed, instead {}",
                forward_speed
            )),
            false => Ok(Self {
                forward_speed,
                ..*self
            }),
        };
    }

    pub fn with_forward_acceleration(&self, forward_acceleration: isize) -> Result<Self> {
        return match forward_acceleration < 1 {
            true => Err(anyhow!(
                "forward acceleration must be strictly positive, instead {}",
                forward_acceleration
            )),
            false => Ok(Self {
                forward_acceleration,
                ..*self
            }),
        };
    }

    pub fn with_lateral_ignorance(&self, lateral_ignorance: f64) -> Result<Self> {
        return match lateral_ignorance <= 0.0 && 1.0 <= lateral_ignorance {
            true => Err(anyhow!(
                "lateral ignorance must be between 0 and 1, instead {}",
                lateral_ignorance
            )),
            false => Ok(Self {
                lateral_ignorance,
                ..*self
            }),
        };
    }
}

impl Default for BikeBuilder {
    fn default() -> Self {
        Self {
            front: 2,
            right: 2,
            length: 2,
            width: 2,
            forward_speed_max: 6,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            lateral_ignorance: 0.2,
        }
    }
}

impl TryInto<Bike> for BikeBuilder {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Bike> {
        return match self.forward_speed_max < self.forward_speed {
            true => Err(anyhow!(
                "forward speed ({}) cannot be greater than max ({})",
                self.forward_speed_max,
                self.forward_speed
            )),
            false => Ok(Bike {
                front: self.front,
                right: self.right,
                length: self.length,
                width: self.width,
                forward_speed_max: self.forward_speed_max,
                forward_speed: self.forward_speed,
                forward_acceleration: self.forward_acceleration,
                rightward_speed_max: self.rightward_speed_max,
                rightward_speed: 0,
                ignore_lateral_distribution: Bernoulli::new(self.lateral_ignorance)?,
            }),
        };
    }
}

#[cfg(test)]
mod tests {

    use rand::distributions::Bernoulli;

    use crate::{
        bike::Bike,
        road::{RectangleOccupier, Road, Vehicle},
    };

    #[test]
    fn bike_can_move_laterally() {
        let bike = Bike {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            rightward_speed: 0,
            ignore_lateral_distribution: Bernoulli::new(0.0).unwrap(),
        };

        let lateral_options: Vec<isize> = bike.potential_lateral_positions().collect();

        assert!(!lateral_options.is_empty())
    }

    #[test]
    fn bike_has_y_prime_empty_road() {
        let bikes = [Bike {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            rightward_speed: 0,
            ignore_lateral_distribution: Bernoulli::new(0.0).unwrap(),
        }];
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();
        let bike_id = 0;

        let y_prime_j_t_plus_1: Vec<RectangleOccupier> = road
            .get_bike(bike_id)
            .y_prime_j_t_plus_1(&road, &bike_id)
            .collect();

        assert!(!y_prime_j_t_plus_1.is_empty());
    }

    #[test]
    fn bike_has_no_collisions_empty_road() {
        let bikes = [Bike {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            rightward_speed: 0,
            ignore_lateral_distribution: Bernoulli::new(0.0).unwrap(),
        }];
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let bike = road.get_bike(0);

        let bike_collides = road.is_collision_for(&bike.rectangle_occupation(), Vehicle::Bike(0));

        assert!(!bike_collides);
    }

    #[test]
    fn bike_is_on_road() {
        let bikes = [Bike {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            rightward_speed: 0,
            ignore_lateral_distribution: Bernoulli::new(0.0).unwrap(),
        }];
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        assert!(road.road_contains_occupier(road.get_bike(0)));
    }

    #[test]
    fn bike_prefers_bl_empty_road() {
        let bikes = [Bike {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            rightward_speed: 0,
            ignore_lateral_distribution: Bernoulli::new(0.0).unwrap(),
        }];
        let mut road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();
        road.update();

        let new_position = road.get_bike(0).rectangle_occupation();

        assert!(!road.motor_lane_contains_occupier(&new_position));
    }
}
