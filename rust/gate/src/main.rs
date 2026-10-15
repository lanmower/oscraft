use std::path::Path;

const TICKS: u32 = 2000;
const MAX_UNITS: usize = 1024;

fn fail(name: &str, detail: &str) -> ! {
    println!("check {name} FAIL {detail}");
    std::process::exit(1);
}

fn run(image: &[u8]) -> Result<(u64, usize), String> {
    let mut sim = sim::load(image, MAX_UNITS).map_err(|e| e.to_string())?;
    for _ in 0..TICKS {
        sim.tick();
    }
    Ok((sim.state_hash(), sim.count()))
}

fn count_spawns(map: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(map).map_err(|e| format!("{}: {e}", map.display()))?;
    Ok(text.lines().filter(|l| l.trim_start().starts_with("spawn(")).count())
}

fn main() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("data");

    let image = osdt_data::generate(&data_dir).unwrap_or_else(|e| fail("image_build", &format!("{e:#}")));
    let (hash, units) = run(&image).unwrap_or_else(|e| fail("sim_load_run", &e));
    let (hash_again, _) = run(&image).unwrap_or_else(|e| fail("sim_load_run", &e));
    let spawns = count_spawns(&data_dir.join("map.star")).unwrap_or_else(|e| fail("map_spawns", &e));

    println!("state_hash={hash:016x} units={units} ticks={TICKS}");

    if hash != hash_again {
        fail("hash_stable_two_runs", &format!("first={hash:016x} second={hash_again:016x}"));
    }
    println!("check hash_stable_two_runs PASS hash={hash:016x}");

    if units < spawns {
        fail("units_ge_map_spawns", &format!("units={units} spawns={spawns}"));
    }
    println!("check units_ge_map_spawns PASS units={units} spawns={spawns}");
}
