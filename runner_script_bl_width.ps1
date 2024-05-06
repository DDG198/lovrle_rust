$env:NUM_ITERATIONS=5000;
$data_dir = ".\gen_data\bl_width_data";
For ($bl_width = 0; $bl_width -lt 14; $bl_width+=2){
For ($num_bikes = 0; $num_bikes -lt 401; $num_bikes+=50){
For ($num_cars = 0; $num_cars -lt 401; $num_cars+=50) {

    Write-Output "setting up environment";
    Set-Location "C:\Users\danie\Documents\Uni\4th Year\FYP\lovrle_rust_v2";

    $env:NUM_BIKES = $num_bikes;
    $env:NUM_CARS = $num_cars;
    $env:BL_WIDTH = $bl_width;
    $env:ML_WIDTH = 14 - $env:BL_WIDTH;

    Set-Location "C:\Users\danie\Documents\Uni\4th Year\FYP\";
    $file_name = "c" + $env:NUM_CARS + "b" + $env:NUM_BIKES + "w" + $env:BL_WIDTH + ".json";
    $out_file = Join-Path -Path $data_dir -ChildPath $file_name;
    Write-Output "setting up environment";

    Write-Output "outfile: " + $outfile;

    if (Test-Path $out_file -PathType Leaf) {
        Write-Output "outfile already exists, skipping";
        continue;
    }

    Write-Output "building executable";
    cargo build --release;
    Write-Output "building complete";

    Write-Output "running executable";
    .\lovrle_rust_v2\target\release\lovrle_rust_v2.exe | Out-File -FilePath $out_file;
    Write-Output "executable complete";

}
}
}
