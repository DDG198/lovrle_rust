use std::{env, fs, path::Path};

fn main() {
    let consts_defaults = [
        ("NUM_BIKES", 200),
        ("NUM_CARS", 200),
        ("LENGTH", 2000),
        ("ML_WIDTH", 7),
        ("BL_WIDTH", 7),
        ("NUM_ITERATIONS", 1000),
    ];

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("constants.rs");

    let mut file_content = String::new();
    for (var_name, default_val) in consts_defaults {
        let value = env::var(var_name)
            .map(|val_str| val_str.parse::<usize>().unwrap())
            .unwrap_or(default_val);
        let this_line = format!("const {}: usize = {};\n", var_name, value);
        file_content.push_str(&this_line);
        println!("cargo::rerun-if-env-changed={}", var_name);
    }

    fs::write(&dest_path, &file_content).unwrap();
    println!("cargo::rerun-if-changed=build.rs");
}
