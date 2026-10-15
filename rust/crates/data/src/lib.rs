//! Generator for OSDT game data. Evaluates the `.star` files of one directory
//! once, validates them, and returns the binary image FORMAT.md describes.
//! The tick path never runs Starlark.

pub mod builtins;
pub mod encode;
pub mod format;
pub mod model;
pub mod reader;
pub mod validate;

use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};

use crate::model::Collector;

/// Evaluates every `*.star` file directly inside `src_dir` (sorted by name),
/// then returns the encoded image. Subdirectories are not read. The image is
/// checked by `reader::verify`, and its sections must read back equal to the
/// sections the encoder wrote, before it is returned.
pub fn generate(src_dir: &Path) -> Result<Vec<u8>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(src_dir)
        .with_context(|| format!("reading {}", src_dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "star"))
        .collect();
    files.sort();

    let globals = builtins::globals();
    let collector = Collector::default();
    for path in &files {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        eval_file(&collector, &globals, &path.display().to_string(), source)?;
    }

    let catalog = validate::validate(collector)?;
    let written = encode::sections(&catalog);
    let image = encode::assemble(&written);
    reader::verify(&image)?;
    check_round_trip(&written, &image)?;
    Ok(image)
}

/// Compares every section read back from the image with the section the
/// encoder wrote: same order, tag, record count and body bytes.
fn check_round_trip(written: &[encode::Section], image: &[u8]) -> Result<()> {
    let read = reader::read_sections(image)?;
    ensure!(
        read.len() == written.len(),
        "round trip: wrote {} sections, read {}",
        written.len(),
        read.len()
    );
    for (w, r) in written.iter().zip(&read) {
        ensure!(
            w.tag == r.tag && w.count == r.count && w.body == r.body,
            "round trip: section {} differs after read-back",
            String::from_utf8_lossy(&w.tag)
        );
    }
    Ok(())
}

fn eval_file(collector: &Collector, globals: &Globals, name: &str, source: String) -> Result<()> {
    let ast = AstModule::parse(name, source, &Dialect::Standard).map_err(|e| anyhow!("{e}"))?;
    Module::with_temp_heap(|module| -> Result<()> {
        let mut eval = Evaluator::new(&module);
        eval.extra = Some(collector);
        eval.eval_module(ast, globals).map_err(|e| anyhow!("{e}"))?;
        Ok(())
    })
}
