use std::io::{stdout, Write};

use lovrle_rust_v2::{
    bike::{Bike, BikeBuilder},
    car::{Car, CarBuilder},
    road::Road,
};

fn main() {
    print!("{{");
    let mut road: Road<200, 200, 2000, 7, 7> = {
        let bikes: Vec<Bike> = (0..200)
            .map(|bike_id| {
                return BikeBuilder::default()
                    .with_front_at(10 * bike_id)
                    .with_right_at(8)
                    .build()
                    .unwrap();
            })
            .collect();
        let cars: Vec<Car> = (0..200)
            .map(|car_id| {
                return CarBuilder::default()
                    .with_front_at(10 * car_id)
                    .build()
                    .unwrap();
            })
            .collect();
        Road::new(
            bikes.try_into().expect("should be right number of bikes"),
            cars.try_into().expect("should be right number of cars"),
        )
        .unwrap()
    };
    print!("\"vehicle_fronts_over_iterations\":[");
    let mut lock = stdout().lock();
    for _iter_num in 0u16..60000 {
        write!(lock, "[{}],", road.vehicle_positions_as_string()).unwrap();
        road.update().unwrap();
    }
    print!("[{}]", road.vehicle_positions_as_string());
    print!("]");
    println!("}}");
}
