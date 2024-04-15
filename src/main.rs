use std::io::{stdout, Write};

use lovrle_rust_v2::{bike::BikeBuilder, car::CarBuilder, road::Road};

fn format_iteration_info(road: &Road<200, 200, 2000, 7, 7>) -> String {
    return format!(
        "{{\"vehicle_fronts\":{},\"mean_car_speed\":{},\"mean_bike_speed\":{}}}",
        road.vehicle_positions_as_string(),
        road.mean_car_speed(),
        road.mean_bike_speed()
    );
}

fn main() {
    print!("{{");
    let mut road: Road<200, 200, 2000, 7, 7> = {
        let bikes: [BikeBuilder; 200] = (0..200)
            .map(|bike_id| {
                return BikeBuilder::default()
                    .with_front_at(10 * bike_id)
                    .with_right_at(8);
            })
            .collect::<Vec<BikeBuilder>>()
            .try_into()
            .expect("should be right number of bikes");
        let cars: [CarBuilder; 200] = (0..200)
            .map(|car_id| return CarBuilder::default().with_front_at(10 * car_id))
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
    print!("\"iterations\":[");
    let mut lock = stdout().lock();
    for _iter_num in 0u16..1000 {
        write!(lock, "{},", format_iteration_info(&road)).unwrap();
        road.update().unwrap();
    }
    // print out final iteration and close the bracket
    print!("{}]", format_iteration_info(&road));
    println!("}}");
}
