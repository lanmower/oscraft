
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use mod_host::{build_image, ModHost};

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))
}

fn manifest(dir: &Path) -> Result<serde_json::Value, String> {
    serde_json::from_str(&read(&dir.join("manifest.json"))?).map_err(|e| format!("manifest.json: {e}"))
}

fn lua_files(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let Some(name) = path.file_name().map(|n| n.to_owned()) else { continue };
        let next = rel.join(&name);
        if path.is_dir() {
            if name != "fixtures" {
                lua_files(&path, &next, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push(next);
        }
    }
    Ok(())
}

fn required(src: &str) -> Vec<String> {
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

fn visit(dir: &Path, files: &[PathBuf], module: &str, out: &mut Vec<(String, String)>) -> Result<(), String> {
    let Some(file) = files.iter().find(|f| f.file_stem().is_some_and(|s| s == module)) else { return Ok(()) };
    let name = file.to_string_lossy().replace('\\', "/");
    if out.iter().any(|(f, _)| *f == name) {
        return Ok(());
    }
    let src = read(&dir.join(file))?;
    let reqs = required(&src);
    out.push((name, src));
    for r in reqs {
        visit(dir, files, &r, out)?;
    }
    Ok(())
}

fn run() -> Result<String, String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: parity_hash <pack-dir> <ticks> <out-dir>";
    let dir = PathBuf::from(args.next().ok_or(usage)?);
    let ticks: u32 = args.next().ok_or(usage)?.parse().map_err(|_| "ticks must be a whole number".to_owned())?;
    let out = PathBuf::from(args.next().ok_or(usage)?);

    let manifest = manifest(&dir)?;
    let rules = read(&dir.join(manifest["rules"].as_str().ok_or("manifest rules file")?))?;
    let roots = manifest["mods"].as_array().ok_or("manifest mods list")?;
    let mut files = Vec::new();
    lua_files(&dir, Path::new(""), &mut files)?;
    files.sort();
    let mut mods = Vec::new();
    for root in roots {
        let stem = Path::new(root.as_str().ok_or("mod file name")?).file_stem().and_then(|s| s.to_str()).ok_or("root module name")?.to_owned();
        visit(&dir, &files, &stem, &mut mods)?;
    }

    let image = build_image(&dir)?;
    let mut host = ModHost::new(&image).map_err(|e| format!("host: {e:?}"))?;
    if !host.sim_mut().set_attr_rules_text(&rules) {
        return Err("the pack's rules text is refused".to_owned());
    }
    host.load_mods(&mods)?;
    for tick in 1..=ticks {
        host.advance().map_err(|e| format!("tick {tick}: {e}"))?;
    }

    let list: Vec<serde_json::Value> = mods.iter().map(|(name, text)| serde_json::json!({ "name": name, "text": text })).collect();
    std::fs::create_dir_all(&out).map_err(|e| format!("create {}: {e}", out.display()))?;
    let put = |name: &str, bytes: &[u8]| std::fs::write(out.join(name), bytes).map_err(|e| format!("write {name}: {e}"));
    put("image.osdt", &image)?;
    put("rules.txt", rules.as_bytes())?;
    put("mods.json", serde_json::Value::Array(list).to_string().as_bytes())?;
    Ok(format!("state_hash={:016x} ticks={ticks}", host.sim().state_hash()))
}

fn main() -> ExitCode {
    match run() {
        Ok(line) => {
            println!("{line}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("parity_hash: {message}");
            ExitCode::FAILURE
        }
    }
}
