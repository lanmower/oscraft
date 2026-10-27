use std::path::Path;

use mod_host::ModHost;

use crate::checks::{pack_dir, RULES_FILE};

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

fn visit(dir: &Path, modules: &serde_json::Map<String, serde_json::Value>, file: &str, out: &mut Vec<(String, String)>) -> Result<(), String> {
    if out.iter().any(|(f, _)| f == file) {
        return Ok(());
    }
    let src = std::fs::read_to_string(dir.join(file)).map_err(|e| format!("read {file}: {e}"))?;
    let reqs = requires(&src);
    out.push((file.to_owned(), src));
    for r in reqs {
        if let Some(next) = modules.get(&r).and_then(|v| v.as_str()) {
            visit(dir, modules, next, out)?;
        }
    }
    Ok(())
}

fn manifest_roots(dir: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(dir.join("manifest.json")).map_err(|e| format!("manifest.json: {e}"))?;
    let manifest: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("manifest.json: {e}"))?;
    let roots = manifest["mods"].as_array().ok_or("manifest.json has no mods list")?;
    roots.iter().map(|m| m.as_str().map(str::to_owned).ok_or_else(|| "a manifest mod is not a file name".to_string())).collect()
}

fn pack_mods(dir: &Path, roots: &[String]) -> Result<Vec<(String, String)>, String> {
    let text = std::fs::read_to_string(dir.join("manifest.json")).map_err(|e| format!("manifest.json: {e}"))?;
    let manifest: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("manifest.json: {e}"))?;
    let modules = manifest["modules"].as_object().ok_or("manifest.json has no modules map")?;
    let mut out = Vec::new();
    for root in roots {
        visit(dir, modules, root, &mut out)?;
    }
    Ok(out)
}

pub fn start() -> Result<ModHost, String> {
    start_with(&[])
}

pub fn start_with(extra: &[(&str, &str)]) -> Result<ModHost, String> {
    let dir = pack_dir();
    let image = osdt_data::generate(&dir).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut host = ModHost::new(&image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    let rules = std::fs::read_to_string(dir.join(RULES_FILE)).map_err(|e| format!("{RULES_FILE}: {e}"))?;
    if !host.sim_mut().set_attr_rules_text(&rules) {
        return Err(format!("{RULES_FILE} has a bad line"));
    }
    host.sim_mut().set_fog(false);
    let mut mods = pack_mods(&dir, &manifest_roots(&dir)?)?;
    mods.extend(extra.iter().map(|(file, src)| ((*file).to_owned(), (*src).to_owned())));
    host.load_mods(&mods).map_err(|e| format!("pack mods: {e}"))?;
    let ui_text = std::fs::read_to_string(dir.join("moba_ui.json")).map_err(|e| format!("moba_ui.json: {e}"))?;
    let ui: serde_json::Value = serde_json::from_str(&ui_text).map_err(|e| format!("moba_ui.json: {e}"))?;
    host.dispatch_action("setup", &serde_json::json!({ "ui": ui, "player": 0 })).map_err(|e| format!("setup: {e}"))?;
    Ok(host)
}

pub fn advance(host: &mut ModHost, ticks: u32) -> Result<(), String> {
    for _ in 0..ticks {
        host.advance().map_err(|e| format!("advance: {e}"))?;
    }
    Ok(())
}

pub fn hurt_to_life(host: &mut ModHost, unit: usize, left: i32) {
    for _ in 0..32 {
        let (life, keep) = {
            let sim = host.sim();
            (sim.hp(unit), i64::from(sim.magic_keep_of(unit)).max(1))
        };
        if life <= left {
            return;
        }
        let needed = (i64::from(life - left) * 100 + keep - 1) / keep;
        host.sim_mut().hurt(unit, needed.min(i64::from(i32::MAX)) as i32);
    }
}
