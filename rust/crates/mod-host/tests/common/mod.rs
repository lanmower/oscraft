#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub fn scenario() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

pub fn pack_roots(dir: &Path) -> Vec<String> {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("manifest")).expect("manifest json");
    manifest["mods"].as_array().expect("manifest mods list").iter().map(|m| m.as_str().expect("mod file name").to_owned()).collect()
}

pub fn pack_rules(dir: &Path) -> String {
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("manifest")).expect("manifest json");
    let file = manifest["rules"].as_str().expect("manifest rules file");
    std::fs::read_to_string(dir.join(file)).unwrap_or_else(|e| panic!("read {file}: {e}"))
}

pub fn pack_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("pack dir") {
            let path = entry.expect("entry").path();
            let name = path.file_name().expect("file name").to_owned();
            let next = rel.join(&name);
            if path.is_dir() {
                if name != "fixtures" {
                    walk(&path, &next, out);
                }
            } else if path.extension().is_some_and(|e| e == "lua") {
                out.push(next);
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, Path::new(""), &mut out);
    out.sort();
    out
}

fn requires(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("require(\"") {
        let after = &rest[i + "require(\"".len()..];
        let Some(j) = after.find('"') else { break };
        out.push(after[..j].to_owned());
        rest = &after[j..];
    }
    out
}

fn visit(dir: &Path, files: &[PathBuf], module: &str, out: &mut Vec<(String, String)>) {
    let Some(file) = files.iter().find(|f| f.file_stem().is_some_and(|s| s == module)) else { return };
    let name = file.to_string_lossy().replace('\\', "/");
    if out.iter().any(|(f, _)| *f == name) {
        return;
    }
    let src = std::fs::read_to_string(dir.join(file)).unwrap_or_else(|e| panic!("read {name}: {e}"));
    let reqs = requires(&src);
    out.push((name, src));
    for r in reqs {
        visit(dir, files, &r, out);
    }
}

pub fn pack_mods(dir: &Path, roots: &[String]) -> Vec<(String, String)> {
    let files = pack_files(dir);
    let mut out = Vec::new();
    for root in roots {
        let stem = Path::new(root).file_stem().and_then(|s| s.to_str()).expect("root module name");
        visit(dir, &files, stem, &mut out);
    }
    out
}
