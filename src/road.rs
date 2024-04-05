use std::{
    collections::HashMap,
    iter::{repeat, zip},
    ops::RangeInclusive,
};

use rand::{seq::SliceRandom, thread_rng};

use anyhow::{anyhow, Result};

use crate::{bike::Bike, car::Car};

#[derive(Debug, PartialEq)]
pub enum Vehicle {
    Bike(usize),
    Car(usize),
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct Coord {
    pub lat: isize,
    pub long: isize,
}

pub trait RoadOccupier {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord>;

    fn occupier_is_within(&self, width: isize) -> bool {
        return self.occupied_cells().any(|Coord { lat, .. }| lat < width);
    }

    fn occupier_is_entirely_within(&self, width: isize) -> bool {
        return self.occupied_cells().all(|Coord { lat, .. }| lat < width);
    }

    fn occupier_is_without(&self, width: isize) -> bool {
        return self.occupied_cells().any(|Coord { lat, .. }| width <= lat);
    }

    fn occupier_is_entirely_without(&self, width: isize) -> bool {
        return self.occupied_cells().all(|Coord { lat, .. }| width <= lat);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
// every occupier is a rectangular occupier so it may make sense
// to do away with the abstraction and just have Bikes and Cars
// contain RectangleOccupiers to track their position and size
pub struct RectangleOccupier {
    pub front: isize,
    pub right: isize,
    pub width: usize,
    pub length: usize,
}

impl RoadOccupier for RectangleOccupier {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        return rectangle_occupation(self.front, self.right, self.width, self.length);
        // return self
        //     .width_iterator()
        //     .map(|lat| zip(repeat(lat), self.length_iterator()))
        //     .flatten()
        //     .map(|(lat, long)| Coord { lat, long });
    }

    // Optimisation: can customise the occupier is within and out implementations
}

pub fn rectangle_occupation(
    front: isize,
    right: isize,
    width: usize,
    length: usize,
) -> impl Iterator<Item = Coord> {
    return (right.saturating_sub_unsigned(width) + 1..=right)
        .map(move |lat| {
            zip(
                repeat(lat),
                front.saturating_sub_unsigned(length) + 1..=front,
            )
        })
        .flatten()
        .map(|(lat, long)| Coord { lat, long });
}

impl RectangleOccupier {
    pub const fn left(&self) -> isize {
        return self.right.saturating_sub_unsigned(self.width) + 1;
    }

    pub const fn back(&self) -> isize {
        return self.front.saturating_sub_unsigned(self.length) + 1;
    }

    pub const fn back_left(&self) -> Coord {
        return Coord {
            lat: self.left(),
            long: self.back(),
        };
    }

    pub fn front_cells(&self) -> impl Iterator<Item = Coord> {
        return zip(self.width_iterator(), repeat(self.front))
            .map(|(lat, long)| Coord { lat, long });
    }

    pub const fn length_iterator(&self) -> RangeInclusive<isize> {
        return self.back()..=self.front;
    }

    pub const fn width_iterator(&self) -> RangeInclusive<isize> {
        return self.left()..=self.right;
    }
}

// constants to preallocate size for the hashmap, can be tuned for performance
const CAR_ALLOCATION: usize = 12;
const BIKE_ALLOCATION: usize = 4;

pub struct RoadCells<const L: usize, const BLW: usize, const MLW: usize> {
    cells: HashMap<Coord, Vehicle>,
}

impl<const L: usize, const BLW: usize, const MLW: usize> RoadCells<L, BLW, MLW> {
    fn empty(capacity: usize) -> Self {
        Self {
            cells: HashMap::with_capacity(capacity),
        }
    }

    fn validate_coord(coord: Coord) -> Result<Coord> {
        let Coord { lat, long } = coord;
        return match lat < Self::total_width() {
            true => Ok(Coord {
                lat,
                long: long.rem_euclid(L as isize),
            }),
            false => Err(anyhow!(
                "lat value {} exceeded total road width {}",
                lat,
                Self::total_width()
            )),
        };
    }

    const fn total_width() -> isize {
        return (BLW + MLW) as isize;
    }

    fn get(&self, coord: &Coord) -> Option<&Vehicle> {
        self.cells.get(coord)
    }

    fn first_car_back(&self, coord: &Coord, maybe_max: Option<usize>) -> Option<&usize> {
        let Coord {
            lat: start_lat,
            long: start_long,
        } = coord;
        // could optimise by keeping track speed of the fastest travelling car,
        // and using that as the max_search distance.
        let max_search = match maybe_max {
            Some(set_max) => set_max as isize,
            None => L as isize,
        };

        return (1isize..max_search)
            .map(|d_long| Coord {
                lat: *start_lat,
                long: start_long - d_long,
            })
            .map(|coord| Self::validate_coord(coord).expect("lat should be in range"))
            .filter_map(|coord| self.get(&coord))
            .find_map(|found_vehicle| match found_vehicle {
                Vehicle::Bike(_) => None,
                Vehicle::Car(found_car_id) => Some(found_car_id),
            });
    }

    fn front_gap(&self, coord: &Coord, maybe_max: Option<usize>) -> Option<isize> {
        let Coord {
            lat: start_lat,
            long: start_long,
        } = *coord;
        let max_search = match maybe_max {
            Some(set_max) => set_max,
            None => L,
        };

        let ahead_coord = (1isize..max_search as isize)
            .map(|d_long| {
                Self::validate_coord(Coord {
                    lat: start_lat,
                    long: start_long + d_long,
                })
                .expect("lat should be in range")
            })
            .find(|coord| self.get(&coord).is_some());

        return ahead_coord.map(
            |Coord {
                 long: found_long, ..
             }| found_long - (start_long + 1),
        );
    }
}

impl<const B: usize, const C: usize, const L: usize, const BLW: usize, const MLW: usize>
    TryFrom<&Road<B, C, L, BLW, MLW>> for RoadCells<L, BLW, MLW>
{
    type Error = anyhow::Error;

    fn try_from(road: &Road<B, C, L, BLW, MLW>) -> Result<Self> {
        let mut cells = HashMap::with_capacity(C * CAR_ALLOCATION + B * BIKE_ALLOCATION);

        road.iter_car_positions()
            .chain(road.iter_bike_positions())
            .try_for_each(
                |(cell, insert_vehicle)| match cells.insert(cell, insert_vehicle) {
                    Some(found_vehicle) => Err(anyhow!(
                        "inserted vehicle {:?} collided with found vehicle {:?} at cell {:?}",
                        cells.get(&cell),
                        found_vehicle,
                        cell
                    )),
                    None => Ok(()),
                },
            )?;

        return Ok(Self { cells });
    }
}

pub struct Road<const B: usize, const C: usize, const L: usize, const BLW: usize, const MLW: usize>
{
    bikes: [Bike; B],
    cars: [Car; C],
    cells: RoadCells<L, BLW, MLW>,
}

impl<const B: usize, const C: usize, const L: usize, const BLW: usize, const MLW: usize>
    Road<B, C, L, BLW, MLW>
{
    pub fn new(bikes: [Bike; B], cars: [Car; C]) -> Result<Self> {
        let mut road = Self {
            bikes,
            cars,
            cells: RoadCells::empty(C * CAR_ALLOCATION + B * BIKE_ALLOCATION),
        };

        road.cells = (&road).try_into()?;

        return Ok(road);
    }

    pub const fn self_total_width(&self) -> isize {
        return Road::<B, C, L, BLW, MLW>::total_width();
    }

    pub const fn total_width() -> isize {
        RoadCells::<L, BLW, MLW>::total_width()
    }

    pub fn iter_car_positions(&self) -> impl Iterator<Item = (Coord, Vehicle)> + '_ {
        return self
            .cars
            .iter()
            .enumerate()
            .map(|(index, car)| zip(car.occupied_cells(), repeat(index)))
            .flatten()
            // not sure if this last line is necessary, as it is clear from the function name
            // that car ids are being returned
            .map(|(cell, car_id)| (cell, Vehicle::Car(car_id)));
    }

    pub fn iter_bike_positions(&self) -> impl Iterator<Item = (Coord, Vehicle)> + '_ {
        return self
            .bikes
            .iter()
            .enumerate()
            .map(|(index, bike)| zip(bike.occupied_cells(), repeat(index)))
            .flatten()
            // same criticism as for iter_car_positions
            .map(|(cell, bike_id)| (cell, Vehicle::Bike(bike_id)));
    }

    fn iter_occupier_positions<const N: usize>(
        occupiers: &[RectangleOccupier; N],
    ) -> impl Iterator<Item = (Coord, usize)> + '_ {
        return occupiers
            .iter()
            .enumerate()
            .map(|(index, occupier)| zip(occupier.occupied_cells(), repeat(index)))
            .flatten()
            .map(|(cell, index)| (cell, index));
    }

    pub fn collisions_for(&self, occupier: &impl RoadOccupier) -> Vec<&Vehicle> {
        return occupier
            .occupied_cells()
            .map(|coord| RoadCells::<L, BLW, MLW>::validate_coord(coord).unwrap())
            .filter_map(|coord| self.cells.get(&coord))
            .collect();
    }

    pub fn is_collision_for(&self, occupier: &impl RoadOccupier, vehicle: Vehicle) -> bool {
        return self
            .collisions_for(occupier)
            .into_iter()
            .any(|found_vehicle| *found_vehicle != vehicle);
    }

    fn bike_lane_contains_occupier(&self, occupier: &impl RoadOccupier) -> bool {
        return occupier.occupier_is_without(MLW as isize);
        // // old implementation, can be tested against
        // occupier
        //     .occupied_cells()
        //     .into_iter()
        //     .map(|(x, y)| x)
        //     .any(|x| (MLW as isize) < x)
    }

    pub fn motor_lane_contains_occupier(&self, occupier: &impl RoadOccupier) -> bool {
        return occupier.occupier_is_within(MLW as isize);
        // // old implementation, can be tested against
        // occupier
        //     .occupied_cells()
        //     .into_iter()
        //     .map(|(x, y)| x)
        //     .any(|x| x >= MLW as isize)
    }

    pub fn road_contains_occupier(&self, occupier: &impl RoadOccupier) -> bool {
        occupier
            .occupied_cells()
            .all(|Coord { lat, .. }| 0 <= lat && lat < Road::<B, C, L, BLW, MLW>::total_width())
    }

    fn vehicle_collides(&self, vehicle: Vehicle) -> bool {
        let occupied_cells: Vec<Coord> = match vehicle {
            Vehicle::Bike(bike_id) => self
                .bikes
                .get(bike_id)
                .expect("bike_id should be valid")
                .occupied_cells()
                .collect(),
            Vehicle::Car(car_id) => self
                .cars
                .get(car_id)
                .expect("car_id should be valid")
                .occupied_cells()
                .collect(),
        };

        return occupied_cells
            .into_iter()
            .map(|coord| RoadCells::<L, BLW, MLW>::validate_coord(coord).unwrap())
            .filter_map(|coord| self.cells.get(&coord))
            .any(|found_vehicle| *found_vehicle != vehicle);
    }

    pub fn get_car(&self, car_id: usize) -> &Car {
        return self.cars.get(car_id).unwrap();
    }

    pub fn get_bike(&self, bike_id: usize) -> &Bike {
        return self.bikes.get(bike_id).unwrap();
    }

    pub fn first_car_back(&self, coord: &Coord, maybe_max: Option<usize>) -> Option<&Car> {
        return match self.cells.first_car_back(coord, maybe_max) {
            Some(car_id) => Some(self.get_car(*car_id)),
            None => None,
        };
    }

    pub fn is_blocking(&self, coord: &Coord, maybe_max: Option<usize>) -> bool {
        return self
            .first_car_back(
                coord, None, // potential optimisation: set reasonable max
            )
            .is_some_and(|car| {
                let distance = car.front() - coord.long;
                return car.next_iteration_potential_speed() < distance;
            });
    }

    pub fn update(&mut self) {
        self.bikes_lateral_update();
        self.bikes_forward_update();
        self.cars_update();
    }

    pub fn bikes_lateral_update(&mut self) {
        let shuffled_new_bikes = {
            let mut rng = thread_rng();
            let mut next_bikes: Vec<(usize, Bike)> =
                self.next_bikes().into_iter().enumerate().collect();
            next_bikes.shuffle(&mut rng);
            next_bikes
        };

        self.wipe_bikes_from_cells();
        for (bike_id, new_bike) in shuffled_new_bikes {
            let bike_to_occupy = match self.collisions_for(&new_bike).is_empty() {
                true => new_bike,
                false => *self.bikes.get(bike_id).expect("should be a valid bike id"),
            };
            bike_to_occupy.occupied_cells().for_each(|occupied_cell| {
                self.cells
                    .cells
                    .insert(occupied_cell, Vehicle::Bike(bike_id));
            });
            self.bikes[bike_id] = bike_to_occupy;
        }
    }

    fn wipe_bikes_from_cells(&mut self) {
        self.bikes
            .iter()
            .map(|bike| bike.occupied_cells())
            .flatten()
            .for_each(|bike_cell| {
                let removed = self.cells.cells.remove(&bike_cell);
                debug_assert!(
                    removed.is_some_and(|vehicle| match vehicle {
                        Vehicle::Bike(_) => true,
                        Vehicle::Car(_) => false,
                    }),
                    "expected to find a bike at this location"
                );
            })
    }

    fn next_bikes(&self) -> [Bike; B] {
        // parallelise me for optimisation
        return self
            .bikes
            .iter()
            .enumerate()
            .map(|(bike_id, bike)| bike.lateral_update(bike_id, self))
            .collect::<Vec<Bike>>()
            .try_into()
            .expect("array length should be okay due to const generic B");
    }

    fn bikes_forward_update(&mut self) {
        todo!()
    }

    fn cars_update(&mut self) {
        todo!()
    }

    pub fn front_gap(&self, occupation: &RectangleOccupier) -> Option<isize> {
        occupation
            .front_cells()
            .filter_map(|coord| self.cells.front_gap(&coord, None))
            .min()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use proptest::{prop_assert_eq, proptest};

    use crate::{
        bike::BikeBuilder,
        proptest_defs::arb_rectangle_occupier,
        road::{Coord, RectangleOccupier, Road, RoadOccupier, Vehicle},
    };

    #[test]
    fn bike_is_on_road() {
        let bikes = [BikeBuilder::default().with_lateral_ignorance(0.0).unwrap()]
            .map(|builder| builder.try_into().unwrap());
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let new_position = road.get_bike(0).rectangle_occupation();

        assert!(road.motor_lane_contains_occupier(&new_position));
    }

    #[test]
    fn bike_is_on_road_after_update() {
        let bikes = [BikeBuilder::default().with_lateral_ignorance(0.0).unwrap()]
            .map(|builder| builder.try_into().unwrap());
        let mut road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        road.update();

        let new_position = road.get_bike(0).rectangle_occupation();

        assert!(road.motor_lane_contains_occupier(&new_position));
    }

    #[test]
    fn multiple_updates_work() {
        let bikes = [BikeBuilder::default().with_lateral_ignorance(0.0).unwrap()]
            .map(|builder| builder.try_into().unwrap());
        let mut road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        for _ in 0..1000 {
            road.update();
        }

        let new_position = road.get_bike(0).rectangle_occupation();

        assert!(road.motor_lane_contains_occupier(&new_position));
    }

    #[test]
    fn rectangle_front_cells_correct() {
        let occupation = RectangleOccupier {
            front: 2,
            right: 5,
            width: 3,
            length: 3,
        };

        let front: Vec<Coord> = occupation.front_cells().collect();

        assert_eq!(
            front,
            vec![
                Coord { lat: 3, long: 2 },
                Coord { lat: 4, long: 2 },
                Coord { lat: 5, long: 2 }
            ]
        )
    }

    #[test]
    fn rectangle_occupies_cells_correct() {
        let width = 2;
        let length = 2;
        let area = (width * length) as usize;
        let occupation = RectangleOccupier {
            front: 2,
            right: 5,
            width,
            length,
        };

        let cells: HashSet<Coord> = occupation.occupied_cells().collect();

        println!("occupier: {:?}", occupation);

        assert_eq!(occupation.occupied_cells().count(), area);
        assert_eq!(cells.len(), area);
        assert_eq!(
            cells,
            HashSet::from([
                Coord { lat: 4, long: 1 },
                Coord { lat: 5, long: 1 },
                Coord { lat: 4, long: 2 },
                Coord { lat: 5, long: 2 }
            ])
        );
    }

    #[test]
    fn front_gap_works() {
        let bikes = [
            BikeBuilder::default().with_front_right_at(Coord { lat: 3, long: 3 }),
            BikeBuilder::default().with_front_right_at(Coord { lat: 3, long: 10 }),
        ]
        .map(|builder| builder.try_into().unwrap());
        let road = Road::<2, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let trailing_bike = road.get_bike(0);

        let front_gap = road
            .front_gap(&trailing_bike.rectangle_occupation())
            .unwrap();

        assert_eq!(front_gap, 5);
    }

    #[test]
    fn cells_front_gap_works() {
        /*
        0 1 2 3 4 5 6 7 8 9 10
              t             l
            x x           x x
                - - - - >
        length 5
        */
        let trailing_coord = Coord { lat: 3, long: 3 };
        let leading_coord = Coord { lat: 3, long: 10 };
        let dimensions = (2, 2);
        let bikes = [
            BikeBuilder::default()
                .with_dimensions(dimensions)
                .unwrap()
                .with_front_right_at(trailing_coord),
            BikeBuilder::default()
                .with_dimensions(dimensions)
                .unwrap()
                .with_front_right_at(leading_coord),
        ]
        .map(|builder| builder.try_into().unwrap());
        let road = Road::<2, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let front_gap = road.cells.front_gap(&trailing_coord, None).unwrap();

        assert_eq!(front_gap, 5);
    }

    #[test]
    fn bike_is_where_expected() {
        /*
        0 1 2 3 4 5 6 7 8 9 10
              t             l
            x x           x x
                - - - - >
        length 5
        */
        let coord = Coord { lat: 3, long: 10 };
        let dimensions = (2, 2);
        let bikes = [BikeBuilder::default()
            .with_dimensions(dimensions)
            .unwrap()
            .with_front_right_at(coord)]
        .map(|builder| builder.try_into().unwrap());
        let road = Road::<1, 0, 20, 3, 3>::new(bikes, []).unwrap();

        let bike = road.get_bike(0);
        let occupied_coords: Vec<Coord> = bike.rectangle_occupation().occupied_cells().collect();

        println!("occupation: {:?}", bike.rectangle_occupation());
        println!("occupied coords: {:?}", occupied_coords);
        println!("length: {}", occupied_coords.len());

        let found_vehicle = road.cells.get(&coord).unwrap();

        assert_eq!(*found_vehicle, Vehicle::Bike(0));
    }

    #[test]
    fn rectangle_occupies_cells_correct_size_larger() {
        let width = 3;
        let length = 4;
        let area = (width * length) as usize;
        let occupier = RectangleOccupier {
            front: 2,
            right: 5,
            width,
            length,
        };
        println!("occupier: {:?}", occupier);

        assert_eq!(occupier.occupied_cells().count(), area);
    }

    #[test]
    fn rectangle_occupier_correct_size() {
        let width = 2;
        let length = 2;
        let area = width * length;
        let occupier = RectangleOccupier {
            front: 2,
            right: 2,
            width,
            length,
        };

        assert_eq!(occupier.occupied_cells().count(), area as usize)
    }

    proptest!(
        #[test]
        fn rectangle_occupier_correct_size_proptest(occupier in arb_rectangle_occupier(-10_000..10_000, -100..100, 10, 10)) {
            let area = occupier.width * occupier.length;
            println!("occupier: {:?}", occupier);
            prop_assert_eq!(occupier.occupied_cells().count(), area)
        }

        #[test]
        fn rectangle_occupier_correct_width_proptest(occupier: RectangleOccupier) {
            println!("occupier: {:?}", occupier);
            prop_assert_eq!(occupier.width_iterator().count(), occupier.width);
        }
    );

    #[test]
    fn rectangle_occupier_correct_size_v2() {
        let width = 2;
        let length = 2;
        let area = (width * length) as usize;
        let occupier = RectangleOccupier {
            front: 2,
            right: 2,
            width,
            length,
        };

        println!("occupier: {:?}", occupier);
        assert_eq!(occupier.occupied_cells().count(), area)
    }

    #[test]
    fn rectangle_width_correct() {
        let width = 2;
        let occupier = RectangleOccupier {
            front: 2,
            right: 2,
            width,
            length: 2,
        };

        assert_eq!(occupier.width_iterator().count(), width as usize)
    }

    #[test]
    fn rectangle_length_correct() {
        let length = 2;
        let occupier = RectangleOccupier {
            front: 2,
            right: 2,
            width: 2,
            length,
        };

        assert_eq!(occupier.length_iterator().count(), length as usize)
    }

    #[test]
    fn positions_on_nm_lane_higher_priority_than_m_lane() {
        let road = Road::<1, 0, 100, 7, 7>::new([BikeBuilder::default().build().unwrap()], []);
    }
}
