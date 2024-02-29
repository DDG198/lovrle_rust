use lovrle_rust_v2::road::Road;

fn main() {
    let road = Road::new(2000, 6, 6, [], []).unwrap();
    println!("{:?}", road.gen_cells().unwrap());
}
