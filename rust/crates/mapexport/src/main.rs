//! Writes the map image a pack loads. Run from rust/: `cargo run -p mapexport -- <package-dir> <out.bin>`.
//! The package's manifest.json lists its .osd files in load order (the `data` list); they compile into one image,
//! the same bytes the spike and gate load.

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: mapexport <package-dir> <out.bin>";
    let dir = PathBuf::from(args.next().expect(usage));
    let out = args.next().expect(usage);
    let manifest_path = dir.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path).unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", manifest_path.display()));
    let data = manifest["data"].as_array().unwrap_or_else(|| panic!("{}: data must be a list", manifest_path.display()));
    let files: Vec<PathBuf> = data
        .iter()
        .map(|v| dir.join(v.as_str().unwrap_or_else(|| panic!("{}: data entries are file names", manifest_path.display()))))
        .collect();
    let image = osdt_data::generate_files(&files).unwrap_or_else(|e| panic!("generate the map image from {}: {e:#}", dir.display()));
    std::fs::write(&out, &image).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {} bytes to {out}", image.len());
}
