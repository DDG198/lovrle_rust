$env:NUM_ITERATIONS=10000;
$data_dir = ".\gen_data\fundamental_data";
For ($num_bikes = 0; $num_bikes -lt 401; $num_bikes+=25){
For ($num_cars = 0; $num_cars -lt 401; $num_cars+=25) {
    Set-Location "C:\Users\danie\Documents\Uni\4th Year\FYP\lovrle_rust_v2";
    Write-Output "building executable";
    $env:NUM_BIKES = $num_bikes;
    $env:NUM_CARS = $num_cars;
    cargo build --release;
    Write-Output "building complete";

    Set-Location "C:\Users\danie\Documents\Uni\4th Year\FYP\";
    Write-Output "running executable";
    $file_name = "c" + $env:NUM_CARS + "b" + $env:NUM_BIKES + ".json";
    $out_file = Join-Path -Path $data_dir -ChildPath $file_name;
    .\lovrle_rust_v2\target\release\lovrle_rust_v2.exe | Out-File -FilePath $out_file;
    Write-Output "executable complete";
}
}
