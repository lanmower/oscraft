pub mod abilityext;
pub mod builtins;
pub mod encode;
pub mod format;
pub mod model;
pub mod persist;
pub mod reader;
pub mod validate;

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};

use crate::model::Collector;

pub fn generate(src_dir: &Path) -> Result<Vec<u8>> {
    generate_files(&definition_files(src_dir)?)
}

fn definition_files(src_dir: &Path) -> Result<Vec<PathBuf>> {
    let manifest_path = src_dir.join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return directory_files(src_dir);
    };
    let manifest: serde_json::Value = serde_json::from_str(&text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;
    let Some(data) = manifest["data"].as_array() else {
        return directory_files(src_dir);
    };
    Ok(data
        .iter()
        .filter_map(|entry| entry.as_str())
        .map(|name| src_dir.join(name))
        .collect())
}

fn directory_files(src_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(src_dir)
        .with_context(|| format!("reading {}", src_dir.display()))?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    if let Some(retired) = entries.iter().find(|path| path.extension().is_some_and(|ext| ext == "star")) {
        return Err(anyhow!(
            "{} has the retired extension star: definition files are .osd, rename it to .osd",
            retired.display()
        ));
    }
    Ok(entries
        .into_iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "osd"))
        .collect())
}

pub fn rules_path(src_dir: &Path) -> Result<Option<PathBuf>> {
    let manifest = src_dir.join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return Ok(None);
    };
    let value: serde_json::Value = serde_json::from_str(&text).with_context(|| format!("parsing {}", manifest.display()))?;
    Ok(value["rules"].as_str().map(|file| src_dir.join(file)))
}

pub fn generate_files(files: &[PathBuf]) -> Result<Vec<u8>> {
    let globals = builtins::globals();
    let collector = Collector::default();
    for path in files {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        eval_file(&collector, &globals, &path.display().to_string(), source)?;
    }

    let catalog = validate::validate(collector)?;
    let image = encode::encode(&catalog);
    reader::verify(&image)?;
    Ok(image)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_uses_manifest_data_order() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
        let text = std::fs::read_to_string(dir.join("manifest.json")).expect("read manifest");
        let manifest: serde_json::Value = serde_json::from_str(&text).expect("parse manifest");
        let files: Vec<PathBuf> = manifest["data"]
            .as_array()
            .expect("data list")
            .iter()
            .map(|entry| dir.join(entry.as_str().expect("file name")))
            .collect();
        let from_manifest = generate_files(&files).expect("generate from manifest order");
        let from_dir = generate(&dir).expect("generate from directory");
        assert_eq!(from_manifest, from_dir, "directory builds must use the manifest data order");
    }
}
