use std::path::{Path, PathBuf};

fn scenario() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("moba")
}

fn scratch(name: &str, tuning: Option<&str>) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("osdt-tuning-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for entry in std::fs::read_dir(scenario()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "osd") && path.file_name().is_some_and(|n| n != "tuning.osd") {
            std::fs::copy(&path, dir.join(path.file_name().unwrap())).unwrap();
        }
    }
    if let Some(text) = tuning {
        std::fs::write(dir.join("tuning.osd"), text).unwrap();
    }
    dir
}

const XP: &str = "200, 500, 900, 1400, 2000, 2600, 3200, 4400, 5400, 6000, 8200, 9000, 10400, 11900, 13500, 15200, 17000, 18900, 20900, 23000, 25200, 27500, 29900, 32400";

fn tuning(points: i32, steps: i32, xp: &str) -> String {
    format!("tuning(structure_scan_cells=3, teleport_ring_points={points}, teleport_ring_steps={steps}, best_only=[], xp_curve=[{xp}])\n")
}

#[test]
fn moba_tuning_generates() {
    let image = osdt_data::generate(&scenario()).expect("moba generates with its tuning");
    assert!(!image.is_empty());
}

#[test]
fn decreasing_xp_curve_is_refused() {
    let xp = XP.replacen("230", "9999", 1);
    let dir = scratch("decreasing", Some(&tuning(8, 6, &xp)));
    let err = osdt_data::generate(&dir).expect_err("a decreasing curve is refused").to_string();
    assert!(err.contains("nondecreasing"), "{err}");
}

#[test]
fn ring_points_past_the_compass_are_refused() {
    let dir = scratch("points", Some(&tuning(9, 6, XP)));
    let err = osdt_data::generate(&dir).expect_err("9 ring points are refused").to_string();
    assert!(err.contains("teleport_ring_points"), "{err}");
}

#[test]
fn missing_tuning_is_refused() {
    let dir = scratch("missing", None);
    let err = osdt_data::generate(&dir).expect_err("a scenario without tuning is refused").to_string();
    assert!(err.contains("no tuning()"), "{err}");
}
