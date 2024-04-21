use std::io::{stdout, Write};

use lovrle_rust_v2::{bike::BikeBuilder, car::CarBuilder, road::Road};

include!(concat!(env!("OUT_DIR"), "/constants.rs"));

const REF: &str = include_str!("../.git/HEAD");
const REF_MASTER: &str = include_str!("../.git/refs/heads/main");

fn format_iteration_info(road: &Road<NUM_BIKES, NUM_CARS, LENGTH, BL_WIDTH, ML_WIDTH>) -> String {
    let car_speed_str = match road.mean_car_speed() {
        None => String::new(),
        Some(car_speed) => format!(",\"mean_car_speed\":{}", car_speed),
    };
    let bike_speed_str = match road.mean_bike_speed() {
        None => String::new(),
        Some(bike_speed) => format!(",\"mean_bike_speed\":{}", bike_speed),
    };
    return format!(
        "{{\"vehicle_fronts\":{}{}{}}}",
        road.vehicle_positions_as_string(),
        car_speed_str,
        bike_speed_str
    );
}

fn main() {
    print!("{{");
    let version = if REF.trim() == "ref: refs/heads/main" {
        REF_MASTER.trim()
    } else {
        REF.trim()
    };
    print!("\"version\":\"{}\",", version);
    let mut road: Road<NUM_BIKES, NUM_CARS, LENGTH, BL_WIDTH, ML_WIDTH> = {
        // no bikes or cars mean the arrays will be empty so the zero spacing
        // won't be a problem
        let bike_spacing = LENGTH.checked_div(NUM_BIKES).unwrap_or(0);
        let car_spacing = LENGTH.checked_div(NUM_CARS).unwrap_or(0);
        let bikes: [BikeBuilder; NUM_BIKES] = (0..NUM_BIKES)
            .map(|bike_id| {
                return BikeBuilder::default()
                    .with_front_at((bike_spacing * bike_id) as isize)
                    .with_right_at((BL_WIDTH + ML_WIDTH) as isize - 1);
            })
            .collect::<Vec<BikeBuilder>>()
            .try_into()
            .expect("should be right number of bikes");
        let cars: [CarBuilder; NUM_CARS] = (0..NUM_CARS)
            .map(|car_id| {
                return CarBuilder::default().with_front_at((car_spacing * car_id) as isize);
            })
            .collect::<Vec<CarBuilder>>()
            .try_into()
            .expect("should be right number of cars");
        print!(
            "\"build_info\":{{\"bikes\":{},\"cars\":{}}},",
            serde_json::to_string(&Into::<Vec<BikeBuilder>>::into(bikes)).unwrap(),
            serde_json::to_string(&Into::<Vec<CarBuilder>>::into(cars)).unwrap(),
        );
        Road::new(
            bikes.map(|builder| builder.build().unwrap()),
            cars.map(|builder| builder.build().unwrap()),
        )
        .unwrap()
    };
    print!(
        "\"road_info\":{{\"num_bikes\":{},\"num_cars\":{},\"length\":{},\"bl_width\":{},\"ml_width\":{},\"num_iterations\":{},\"car_density\":{},\"bike_density\":{}}},",
        NUM_BIKES,
        NUM_CARS,
        LENGTH,
        BL_WIDTH,
        ML_WIDTH,
        NUM_ITERATIONS,
        road.car_density(),
        road.bike_density()
    );
    print!("\"iterations\":[");
    let mut lock = stdout().lock();
    for _iter_num in 0..NUM_ITERATIONS {
        write!(lock, "{},", format_iteration_info(&road)).unwrap();
        road.update().unwrap();
    }
    // print out final iteration and close the bracket
    print!("{}]", format_iteration_info(&road));
    println!("}}");
}
