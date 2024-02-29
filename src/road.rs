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

pub struct Road<const B: usize, const C: usize> {
    length: usize,
    ml_width: usize,
    bl_width: usize,
    bikes: [Bike; B],
    cars: [Car; C],
    cells: HashMap<(isize, isize), Vehicle>,
}

impl<const B: usize, const C: usize> Road<B, C> {
    pub fn new(
        length: usize,
        ml_width: usize,
        bl_width: usize,
        bikes: [Bike; B],
        cars: [Car; C],
    ) -> Result<Self> {
        let mut road = Self {
            length,
            ml_width,
            bl_width,
            bikes,
            cars,
            cells: HashMap::with_capacity(C * CAR_ALLOCATION + B * BIKE_ALLOCATION),
        };

        road.cells = road.gen_cells()?;

        return Ok(road);
    }

    pub fn gen_cells(&self) -> Result<HashMap<(isize, isize), Vehicle>> {
        let mut cells = HashMap::with_capacity(C * CAR_ALLOCATION + B * BIKE_ALLOCATION);

        self.cars
            .iter()
            .enumerate()
            .map(|(index, car)| zip(car.occupied_cells(), repeat(index)))
            .flatten()
            .try_for_each(
                |(cell, index)| match cells.insert(cell, Vehicle::Car(index)) {
                    Some(vehicle) => Err(anyhow!(
                        "inserted car {:?} collided with vehicle {:?}",
                        index,
                        vehicle
                    )),
                    None => Ok(()),
                },
            )?;

        self.bikes
            .iter()
            .enumerate()
            .map(|(index, bike)| zip(bike.occupied_cells(), repeat(index)))
            .flatten()
            .try_for_each(
                |(cell, index)| match cells.insert(cell, Vehicle::Bike(index)) {
                    Some(vehicle) => Err(anyhow!(
                        "inserted bike {:?} collided with vehicle {:?}",
                        index,
                        vehicle
                    )),
                    None => Ok(()),
                },
            )?;

        return Ok(cells);
    }
}
