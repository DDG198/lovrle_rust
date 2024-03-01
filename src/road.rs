use std::{
    collections::HashMap,
    iter::{repeat, zip},
};

use anyhow::{anyhow, Result};

use crate::{bike::Bike, car::Car};

#[derive(Debug)]
pub enum Vehicle {
    Bike(usize),
    Car(usize),
}

pub trait RoadOccupier {
    fn occupied_cells(&self) -> impl IntoIterator<Item = (isize, isize)>;
}

const CAR_ALLOCATION: usize = 12;
const BIKE_ALLOCATION: usize = 4;

pub struct RoadCells<const L: usize, const BLW: usize, const MLW: usize> {
    cells: HashMap<(isize, isize), Vehicle>,
}

impl<const L: usize, const BLW: usize, const MLW: usize> RoadCells<L, BLW, MLW> {
    fn empty(capacity: usize) -> Self {
        Self {
            cells: HashMap::with_capacity(capacity),
        }
    }

    // Maybe would be best for the Road struct to handle this, based on the id
    // on a Vehicle variant? This way, handling self-collisions is simple as we
    // already would know the id of things we want to ignore.
    fn collides(&self, vehicle: impl RoadOccupier) -> bool {
        return vehicle
            .occupied_cells()
            .into_iter()
            // maybe better to handle the error here instead of unwrapping
            // ideally, this wouldn't fail as vehicles would always stay within
            // bounds
            .map(|(x, y)| Self::validate_coord(x, y).unwrap())
            .any(|occupied_cell| self.cells.contains_key(&occupied_cell));
    }

    fn validate_coord(x: isize, y: isize) -> Result<(isize, isize)> {
        match y < Self::total_width() {
            true => Ok((x.rem_euclid(L as isize), y)),
            false => Err(anyhow!(
                "y value {} exceeded total road width {}",
                y,
                Self::total_width()
            )),
        }
    }

    const fn total_width() -> isize {
        return (BLW + MLW) as isize;
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
                        "inserted vehicle {:?} collided with found vehicle {:?}",
                        insert_vehicle,
                        found_vehicle
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
    pub fn new(
        length: usize,
        ml_width: usize,
        bl_width: usize,
        bikes: [Bike; B],
        cars: [Car; C],
    ) -> Result<Self> {
        let mut road = Self {
            bikes,
            cars,
            cells: RoadCells::empty(C * CAR_ALLOCATION + B * BIKE_ALLOCATION),
        };

        road.cells = (&road).try_into()?;

        return Ok(road);
    }

    pub fn iter_car_positions(&self) -> impl Iterator<Item = ((isize, isize), Vehicle)> + '_ {
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

    pub fn iter_bike_positions(&self) -> impl Iterator<Item = ((isize, isize), Vehicle)> + '_ {
        return self
            .bikes
            .iter()
            .enumerate()
            .map(|(index, bike)| zip(bike.occupied_cells(), repeat(index)))
            .flatten()
            // same criticism as for iter_car_positions
            .map(|(cell, bike_id)| (cell, Vehicle::Bike(bike_id)));
    }

    pub fn update(&mut self) {
        self.bikes_lateral_update();
        self.bikes_forward_update();
        self.cars_update();
    }

    fn bikes_lateral_update(&self) {
        let next_bikes: [Bike; B] = self
            .bikes
            .iter()
            .map(|bike| bike.lateral_update(&self.cells))
            .collect();
    }

    fn bikes_forward_update(&self) {
        todo!()
    }

    fn cars_update(&self) {
        todo!()
    }
}
