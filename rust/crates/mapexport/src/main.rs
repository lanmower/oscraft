//! Writes the map image the browser loads. Run from rust/: `cargo run -p mapexport -- <out.bin>`.
//! The image is built from the .osd sources in data/, the same bytes the spike and gate load.

use std::path::Path;

fn main() {
    let out = std::env::args().nth(1).expect("usage: mapexport <out.bin>");
    let image = osdt_data::generate(Path::new("data")).expect("generate the map image from data/");
    std::fs::write(&out, &image).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {} bytes to {out}", image.len());
}
