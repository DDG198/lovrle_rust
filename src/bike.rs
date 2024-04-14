use std::cmp::{max, Ordering};

use anyhow::{anyhow, Ok, Result};
use rand::{
    distributions::Bernoulli,
    prelude::{Distribution, IteratorRandom},
};

use crate::road::{Coord, RectangleOccupier, Road, RoadOccupier, Vehicle};

#[derive(Debug, Copy, Clone)]
pub enum YStarSelectionStrategy {
    Rightmost,
    UniformRandom,
}

#[derive(Debug, Copy, Clone)]
pub struct Bike {
    occupation: RectangleOccupier,
    forward_speed_max: isize,
    pub forward_speed: isize,
    forward_acceleration: isize,
    rightward_speed_max: isize,
    ignore_lateral_distribution: Bernoulli,
    decelerate_distribution: Bernoulli,
    y_star_selection_strategy: YStarSelectionStrategy,
}

impl Bike {
    pub const fn front(&self) -> isize {
        return self.occupation.front;
    }

    /// Returns the positions that the bike could move to laterally
    pub const fn potential_lateral_positions(&self) -> impl Iterator<Item = isize> {
        // could add something to do with the width of the bike here,
        // ensuring that the lhs of the bike is not off the road.
        // Could also put a similar check in for the right side? but
        // that would require knowledge of the road width.
        // Leave this as an optimisation for the future.
        return (self.occupation.right - self.rightward_speed_max)
            ..(self.occupation.right + self.rightward_speed_max + 1);
    }

    fn should_ignore_lateral_movement(&self) -> bool {
        return self
            .ignore_lateral_distribution
            .sample(&mut rand::thread_rng());
    }

    fn should_decelerate(&self) -> bool {
        return self.decelerate_distribution.sample(&mut rand::thread_rng());
    }

    fn y_j_t_plus_1(&self) -> impl Iterator<Item = isize> {
        return self.potential_lateral_positions();
    }

    pub fn lateral_update<
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
        } else {
            return Self {
                occupation: self.select_y_star(road, self_id),
                ..*self
            };
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
                right: position,
                ..self.occupation
            })
            // check that the occupation is on the road
            .filter(|occupation| road.road_contains_occupier(occupation))
            // check that the spaces are free
            .filter(|occupation| !road.is_collision_for(occupation, Vehicle::Bike(*self_id)));
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
                // both on motor lane
                (true, true) => lhs.left().cmp(&rhs.left()),
                (true, false) => Ordering::Less,    // lhs < rhs
                (false, true) => Ordering::Greater, // lhs > rhs
                // both on bike lane
                (false, false) => Ordering::Equal,
            },
            Ordering::Greater => Ordering::Greater,
        }
    }

    // fn avoid_blocking_ypp_filter<
    //     'a,
    //     const B: usize,
    //     const C: usize,
    //     const L: usize,
    //     const BLW: usize,
    //     const MLW: usize,
    // >(
    //     yp: impl Iterator<Item = RectangleOccupier> + 'a,
    //     road: &'a Road<B, C, L, BLW, MLW>,
    //     boundary: isize,
    // ) -> impl Iterator<Item = RectangleOccupier> + '_ {
    //     yp.filter(
    //         move |occupation| match occupation.occupier_is_within(boundary) {
    //             true => road.is_blocking(&occupation.back_left(), None),
    //             false => true,
    //         },
    //     )
    // }

    pub const fn rectangle_occupation(&self) -> RectangleOccupier {
        return self.occupation;
    }

    fn y_prime_prime_j_t_plus_1<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
        self_id: usize,
    ) -> Vec<RectangleOccupier> {
        return y_prime_prime_j_t_plus_1(
            &road,
            self.rectangle_occupation(),
            self.y_prime_j_t_plus_1(road, &self_id),
        )
        .into_iter()
        .collect();
    }

    fn generate_y_stars<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
        self_id: usize,
    ) -> Vec<RectangleOccupier> {
        let mut y_prime_prime = self.y_prime_prime_j_t_plus_1(road, self_id);
        y_prime_prime.sort_by(|lhs, rhs| Bike::y_star_cmp_priority(road, lhs, rhs));
        let best_choice_example = match y_prime_prime.first() {
            Some(choice) => choice,
            None => return Vec::new(), // nothing to choose y_stars from so just return nothing
        }
        .clone();
        let best_choices = y_prime_prime
            .into_iter()
            // keep the ones that have priority equal with the first element
            .take_while(|choice| {
                Bike::y_star_cmp_priority(road, &best_choice_example, choice).is_eq()
            });
        return best_choices.collect();
    }

    fn select_y_star<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
        self_id: usize,
    ) -> RectangleOccupier {
        let y_prime_prime = self.y_prime_prime_j_t_plus_1(road, self_id);
        return match self.y_star_selection_strategy {
            YStarSelectionStrategy::Rightmost => rightmost_y_star_selector(y_prime_prime),
            YStarSelectionStrategy::UniformRandom => uniform_y_star_selector(y_prime_prime),
        }
        // staying still is valid if nothing else is found to be
        .unwrap_or(self.occupation);
    }

    pub fn forward_update<
        const B: usize,
        const C: usize,
        const L: usize,
        const BLW: usize,
        const MLW: usize,
    >(
        &self,
        road: &Road<B, C, L, BLW, MLW>,
    ) -> Self {
        let next_speed = [
            // try and accelerate
            self.forward_speed + self.forward_acceleration,
            // unless that is too fast
            self.forward_speed_max,
            // unless you'd crash by going that fast
            road.front_gap(&self.rectangle_occupation())
                .expect("bike should have width")
                .try_into()
                .expect("shouldn't be too large"),
        ]
        .into_iter()
        .min()
        .expect("iterator should have 3 values");

        let next_speed = match self.should_decelerate() {
            false => next_speed,
            true => max(next_speed - 1, 0),
        };

        let next_occupation = RectangleOccupier {
            front: (self.occupation.front + next_speed).rem_euclid(L as isize),
            ..self.occupation
        };

        return Self {
            occupation: next_occupation,
            forward_speed: next_speed,
            ..*self
        };
    }
}

fn rightmost_y_star_selector(
    options: impl IntoIterator<Item = RectangleOccupier>,
) -> Option<RectangleOccupier> {
    return options
        .into_iter()
        .max_by_key(|&RectangleOccupier { right, .. }| right);
}

fn uniform_y_star_selector(
    options: impl IntoIterator<Item = RectangleOccupier>,
) -> Option<RectangleOccupier> {
    return options.into_iter().choose(&mut rand::thread_rng());
    // let selected_index = (0..options.len())
    //     .choose(&mut rand::thread_rng())?
    // return options
    //     .remove(selected_index);
}

fn y_prime_prime_j_t_plus_1<
    const B: usize,
    const C: usize,
    const L: usize,
    const BLW: usize,
    const MLW: usize,
>(
    road: &Road<B, C, L, BLW, MLW>,
    current_occupation: RectangleOccupier,
    y_prime_j_t_plus_1: impl Iterator<Item = RectangleOccupier>,
) -> Vec<RectangleOccupier> {
    return match determine_y_prime_prime_j_t_plus_1_filter(road, current_occupation) {
        YPrimePrimeFilter::MotorLaneBlocking => {
            y_prime_prime_motor_lane_blocking(y_prime_j_t_plus_1, road)
        }
        YPrimePrimeFilter::MotorLaneNonBlocking => {
            avoid_blocking_ypp_filter(y_prime_j_t_plus_1, road, current_occupation.right).collect()
        }
        YPrimePrimeFilter::BikeLane => {
            avoid_blocking_ypp_filter(y_prime_j_t_plus_1, road, current_occupation.right).collect()
        }
    };
}

#[derive(Debug, PartialEq)]
enum YPrimePrimeFilter {
    MotorLaneBlocking,
    MotorLaneNonBlocking,
    BikeLane,
}

fn determine_y_prime_prime_j_t_plus_1_filter<
    const B: usize,
    const C: usize,
    const L: usize,
    const BLW: usize,
    const MLW: usize,
>(
    road: &Road<B, C, L, BLW, MLW>,
    current_occupation: RectangleOccupier,
) -> YPrimePrimeFilter {
    return match road.motor_lane_contains_occupier(&current_occupation) {
        true => match road.is_blocking(&current_occupation.back_left(), None) {
            true => YPrimePrimeFilter::MotorLaneBlocking,
            false => YPrimePrimeFilter::MotorLaneNonBlocking,
        },
        false => YPrimePrimeFilter::BikeLane,
    };
}

fn y_prime_prime_motor_lane_blocking<
    const B: usize,
    const C: usize,
    const L: usize,
    const BLW: usize,
    const MLW: usize,
>(
    y_prime_j_t_plus_1: impl Iterator<Item = RectangleOccupier>,
    road: &Road<B, C, L, BLW, MLW>,
) -> Vec<RectangleOccupier> {
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
            true => !road.is_blocking(&occupation.back_left(), None),
            false => true,
        },
    )
}

// fn select_y_star<
//     const B: usize,
//     const C: usize,
//     const L: usize,
//     const BLW: usize,
//     const MLW: usize,
// >(
//     choices: Vec<RectangleOccupier>,
//     road: &Road<B, C, L, BLW, MLW>,
// ) -> Vec<RectangleOccupier> {
//     choices.sort_by(|lhs, rhs| Bike::y_star_cmp_priority(road, lhs, rhs));
//     let best_choice_example = match choices.first() {
//         Some(choice) => choice,
//         None => return Vec::new(), // nothing to choose y_stars from so just return nothing
//     };
//     let best_choices = choices
//         .into_iter()
//         // keep the ones that have priority equal with the first element
//         .take_while(|choice| Bike::y_star_cmp_priority(road, best_choice_example, choice).is_eq());
//     return best_choices.collect();
// }

impl RoadOccupier for Bike {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        return self.occupation.occupied_cells();
    }
}

impl Default for Bike {
    fn default() -> Self {
        return BikeBuilder::default()
            .build()
            .expect("default bike builder configuration should be valid");
    }
}

#[derive(Debug)]
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
    deceleration_prob: f64,
    y_star_selection_strategy: YStarSelectionStrategy,
}

impl BikeBuilder {
    pub fn deterministic_default() -> Self {
        Self {
            lateral_ignorance: 0.0,
            deceleration_prob: 0.0,
            y_star_selection_strategy: YStarSelectionStrategy::Rightmost,
            ..Default::default()
        }
    }

    pub const fn with_front_at(&self, front: isize) -> Self {
        return Self { front, ..*self };
    }

    pub const fn with_right_at(&self, right: isize) -> Self {
        return Self { right, ..*self };
    }

    pub const fn with_front_right_at(&self, front_right: Coord) -> Self {
        let Coord {
            lat: right,
            long: front,
        } = front_right;
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

    pub fn with_rightward_speed_max(&self, rightward_speed_max: isize) -> Result<Self> {
        return match rightward_speed_max.is_negative() {
            true => Err(anyhow!(
                "cannot have negative max speed, instead {}",
                rightward_speed_max
            )),
            false => Ok(Self {
                rightward_speed_max,
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

    pub fn with_deceleration_prob(&self, deceleration_prob: f64) -> Result<Self> {
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

    pub fn with_y_star_selection_strategy(
        &self,
        y_star_selection_strategy: YStarSelectionStrategy,
    ) -> Self {
        return Self {
            y_star_selection_strategy,
            ..*self
        };
    }

    pub fn build(&self) -> Result<Bike> {
        return self.try_into();
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
            deceleration_prob: 0.2,
            y_star_selection_strategy: YStarSelectionStrategy::UniformRandom,
        }
    }
}

impl TryInto<Bike> for &BikeBuilder {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Bike> {
        return match self.forward_speed_max < self.forward_speed {
            true => Err(anyhow!(
                "forward speed ({}) cannot be greater than max ({})",
                self.forward_speed_max,
                self.forward_speed
            )),
            false => Ok(Bike {
                occupation: RectangleOccupier {
                    front: self.front,
                    right: self.right,
                    length: self.length.try_into()?,
                    width: self.width.try_into()?,
                },
                forward_speed_max: self.forward_speed_max,
                forward_speed: self.forward_speed,
                forward_acceleration: self.forward_acceleration,
                rightward_speed_max: self.rightward_speed_max,
                ignore_lateral_distribution: Bernoulli::new(self.lateral_ignorance)?,
                decelerate_distribution: Bernoulli::new(self.deceleration_prob)?,
                y_star_selection_strategy: self.y_star_selection_strategy,
            }),
        };
    }
}

impl TryInto<Bike> for BikeBuilder {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Bike> {
        return (&self).try_into();
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        bike::{
            determine_y_prime_prime_j_t_plus_1_filter, y_prime_prime_j_t_plus_1, Bike, BikeBuilder,
            YPrimePrimeFilter, YStarSelectionStrategy,
        },
        road::{RectangleOccupier, Road, Vehicle},
    };

    #[test]
    fn bike_can_move_laterally() {
        let bike: Bike = BikeBuilder {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap();

        let lateral_options: Vec<isize> = bike.potential_lateral_positions().collect();

        assert!(!lateral_options.is_empty())
    }

    #[test]
    fn bike_has_y_prime_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
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
        let bikes = [BikeBuilder {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let bike = road.get_bike(0);

        let bike_collides = road.is_collision_for(&bike.rectangle_occupation(), Vehicle::Bike(0));

        assert!(!bike_collides);
    }

    #[test]
    fn bike_is_on_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 3,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 2,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        assert!(road.road_contains_occupier(road.get_bike(0)));
    }

    #[test]
    fn bike_is_ml_non_blocking_empty_road_no_bl() {
        let bikes = [BikeBuilder::default()
            .with_lateral_ignorance(0.0)
            .unwrap()
            .build()
            .unwrap()];
        let road = Road::<1, 0, 20, 0, 6>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);

        let filter_type =
            determine_y_prime_prime_j_t_plus_1_filter(&road, bike.rectangle_occupation());
        assert_eq!(filter_type, YPrimePrimeFilter::MotorLaneNonBlocking);
    }

    #[test]
    fn y_prime_prime_is_y_prime_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            // high enough to move anywhere on the road
            rightward_speed_max: 20,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 10, 10>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);
        let y_prime: Vec<RectangleOccupier> = bike.y_prime_j_t_plus_1(&road, &0).collect();
        let y_prime_prime: Vec<RectangleOccupier> = bike.y_prime_prime_j_t_plus_1(&road, 0);
        // y_prime_prime_j_t_plus_1(&road, bike.rectangle_occupation(), y_prime.into_iter())
        //     .into_iter()
        //     .collect();

        assert_eq!(y_prime, y_prime_prime);
    }

    #[test]
    fn y_star_expected_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            // high enough to move anywhere on the road
            rightward_speed_max: 20,
            lateral_ignorance: 0.0,
            y_star_selection_strategy: YStarSelectionStrategy::Rightmost,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 10, 10>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);
        let y_star_right = bike.select_y_star(&road, 0).right;
        assert_eq!(y_star_right, road.self_total_width() - 1);
    }

    #[test]
    fn zero_ignorance_never_ignores() {
        let bike = BikeBuilder::default()
            .with_lateral_ignorance(0.0)
            .unwrap()
            .build()
            .unwrap();

        assert!(!bike.should_ignore_lateral_movement())
    }

    #[test]
    fn one_ignorance_always_ignores() {
        let bike = BikeBuilder::default()
            .with_lateral_ignorance(1.0)
            .unwrap()
            .build()
            .unwrap();

        assert!(bike.should_ignore_lateral_movement())
    }

    #[test]
    fn zero_deceleration_prob_never_decelerates() {
        let bike = BikeBuilder::default()
            .with_deceleration_prob(0.0)
            .unwrap()
            .build()
            .unwrap();

        assert!(!bike.should_decelerate())
    }

    #[test]
    fn one_deceleration_prob_always_decelerates() {
        let bike = BikeBuilder::default()
            .with_deceleration_prob(1.0)
            .unwrap()
            .build()
            .unwrap();

        assert!(bike.should_decelerate())
    }

    #[test]
    fn y_expected_empty_road() {
        let bike = BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 5,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap();

        let y: Vec<isize> = bike.y_j_t_plus_1().collect();
        assert_eq!(y, vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]);
    }

    #[test]
    fn y_prime_expected_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 5,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 10, 10>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);

        let y_prime: Vec<RectangleOccupier> = bike.y_prime_j_t_plus_1(&road, &0).collect();
        let expected_occupations: Vec<RectangleOccupier> = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
            .map(|right| RectangleOccupier {
                right,
                ..bike.rectangle_occupation()
            })
            .into();

        assert_eq!(y_prime, expected_occupations);
    }

    #[test]
    fn y_prime_prime_expected_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 5,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 10, 10>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);

        let y_prime_prime: Vec<RectangleOccupier> = y_prime_prime_j_t_plus_1(
            &road,
            bike.rectangle_occupation(),
            bike.y_prime_j_t_plus_1(&road, &0),
        );
        let expected_occupations: Vec<RectangleOccupier> = [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
            .map(|right| RectangleOccupier {
                right,
                ..bike.rectangle_occupation()
            })
            .into();

        assert_eq!(y_prime_prime, expected_occupations);
    }

    #[test]
    fn y_prime_prime_type_expected_empty_road() {
        let bikes = [BikeBuilder {
            front: 3,
            right: 9,
            length: 2,
            width: 2,
            forward_speed_max: 5,
            forward_speed: 0,
            forward_acceleration: 1,
            rightward_speed_max: 5,
            lateral_ignorance: 0.0,
            ..Default::default()
        }
        .build()
        .unwrap()];
        let road = Road::<1, 0, 20, 10, 10>::new(bikes, []).unwrap();
        let bike = road.get_bike(0);

        let y_prime_prime_type: YPrimePrimeFilter =
            determine_y_prime_prime_j_t_plus_1_filter(&road, bike.rectangle_occupation());

        assert_eq!(y_prime_prime_type, YPrimePrimeFilter::MotorLaneNonBlocking);
    }
}
