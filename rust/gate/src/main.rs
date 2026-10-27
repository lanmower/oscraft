mod checks;
mod claimed_checks;
mod mod_checks;
mod mpq_roundtrip;
mod sim_checks;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("mpq") {
        if !mpq_roundtrip::run() {
            std::process::exit(1);
        }
        return;
    }

    let image = match osdt_data::generate(&checks::data_dir()) {
        Ok(image) => image,
        Err(e) => {
            println!("check image_build FAIL {e:#}");
            std::process::exit(1);
        }
    };
    println!("check image_build PASS bytes={}", image.len());

    let (hash, units) = checks::state_after(&image, checks::RUN_TICKS).unwrap_or_else(|e| {
        println!("check sim_load_run FAIL {e}");
        std::process::exit(1);
    });
    println!("state_hash={hash:016x} units={units} ticks={}", checks::RUN_TICKS);

    let failed = checks::run_all(&image);
    println!("gate: {} checks, {failed} failed", checks::CHECKS.len());
    if failed > 0 {
        std::process::exit(1);
    }
}
