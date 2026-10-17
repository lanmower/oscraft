//! Writes the map image the browser loads. Run from rust/: `cargo run -p mapexport -- <out.bin> [data-dir]`.
//! The image is built from the .osd sources in data-dir (default data/), the same bytes the spike and gate load.

use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: mapexport <out.bin> [data-dir]");
    let dir = args.next().unwrap_or_else(|| "data".to_owned());
    let image = osdt_data::generate(Path::new(&dir)).unwrap_or_else(|e| panic!("generate the map image from {dir}: {e:#}"));
    std::fs::write(&out, &image).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {} bytes to {out}", image.len());
}
