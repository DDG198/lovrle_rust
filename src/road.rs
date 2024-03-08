use std::{
    collections::HashMap,
    iter::{repeat, zip},
};

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
        return self.occupied_cells().any(|Coord { lat, .. }| width < lat);
    }

    fn occupier_is_without(&self, width: isize) -> bool {
        return self.occupied_cells().any(|Coord { lat, .. }| lat <= width);
    }
}

#[derive(Clone, Copy)]
// every occupier is a rectangular occupier so it may make sense
// to do away with the abstraction and just have Bikes and Cars
// contain RectangleOccupiers to track their position and size
pub struct RectangleOccupier {
    pub front: isize,
    pub right: isize,
    pub width: isize,
    pub length: isize,
}

impl RoadOccupier for RectangleOccupier {
    fn occupied_cells(&self) -> impl Iterator<Item = Coord> {
        return (self.right..(self.right + self.width))
            .map(|lat| zip(repeat(lat), (self.front - self.length)..(self.front)))
            .flatten()
            .map(|(lat, long)| Coord { lat, long });
    }

    // Optimisation: can customise the occupier is within and out implementations
}

impl RectangleOccupier {
    pub const fn left(&self) -> isize {
        return self.right - self.width;
    }

    pub const fn back(&self) -> isize {
        return self.front - self.length;
    }

    pub const fn back_left(&self) -> Coord {
        return Coord {
            lat: self.left(),
            long: self.back(),
        };
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
        occupier.occupier_is_within(MLW as isize)
        // // old implementation, can be tested against
        // occupier
        //     .occupied_cells()
        //     .into_iter()
        //     .map(|(x, y)| x)
        //     .any(|x| (MLW as isize) < x)
    }

    pub fn motor_lane_contains_occupier(&self, occupier: &impl RoadOccupier) -> bool {
        occupier.occupier_is_without(MLW as isize)
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
        // self.bikes_forward_update();
        // self.cars_update();
    }

    fn bikes_lateral_update(&self) {
        let _new_bikes: [Bike; B] = self
            .bikes
            .iter()
            .enumerate()
            .map(|(bike_id, bike)| bike.self_lateral_update(bike_id, self))
            .collect::<Vec<Bike>>()
            .try_into()
            .expect("array length should be okay due to const generic B");
        todo!()
    }

    fn bikes_forward_update(&self) {
        todo!()
    }

    fn cars_update(&self) {
        todo!()
    }

    pub fn front_gap(&self, occupation: &RectangleOccupier) -> usize {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::{bike::BikeBuilder, road::Road};

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
}
