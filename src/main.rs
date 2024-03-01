use lovrle_rust_v2::road::Road;

fn main() {
    let road: Road<0, 0, 16, 4, 4> = Road::new([], []).unwrap();
    println!("{:?}", road.iter_car_positions().collect::<Vec<_>>());
}
