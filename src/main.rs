use std::io::{stdout, Write};

use lovrle_rust_v2::{bike::BikeBuilder, car::CarBuilder, road::Road};

include!(concat!(env!("OUT_DIR"), "/constants.rs"));

fn format_iteration_info(road: &Road<NUM_BIKES, NUM_CARS, LENGTH, BL_WIDTH, ML_WIDTH>) -> String {
    return format!(
        "{{\"vehicle_fronts\":{},\"mean_car_speed\":{},\"mean_bike_speed\":{}}}",
        road.vehicle_positions_as_string(),
        road.mean_car_speed(),
        road.mean_bike_speed()
    );
}

fn main() {
    print!("{{");
    let mut road: Road<NUM_BIKES, NUM_CARS, LENGTH, BL_WIDTH, ML_WIDTH> = {
        let bike_spacing = LENGTH / NUM_BIKES;
        let car_spacing = LENGTH / NUM_CARS;
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
