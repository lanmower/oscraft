//! Hot reload harness. Swaps a mod in at the tick boundary, then shows a broken mod is
//! refused with its file and line while the previous version keeps running. CI greps the
//! three `reload-check` lines this prints.

use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime};

use mod_host::ModHost;

const GOOD_V1: &str = "function on_tick(state)\n  state.value = 1\nend\n";
const GOOD_V2: &str = "function on_tick(state)\n  state.value = 2\nend\n";
const BROKEN: &str = "function on_tick(state)\n  state.value = \nend\n";
// Side effects at top level, then a runtime error: the chunk compiles, runs its spawn,
// region and timer, and only then fails.
const SIDE_EFFECTS_THEN_ERROR: &str = "spawn_unit(\"skirmisher\", 1343488, 1343488)\ndefine_region(\"ghost\", 0, 0, 655360, 655360)\nschedule(\"ghost\", 1, function() end)\nerror(\"boom\")\n";

// reload() keys on mtime. Each write moves mtime forward by a step so the edit is seen
// without sleeping.
fn write_mod(path: &Path, src: &str, step: u64) -> Result<(), String> {
    fs::write(path, src).map_err(|e| format!("write {}: {e}", path.display()))?;
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    file.set_modified(SystemTime::now() + Duration::from_secs(120 + 60 * step))
        .map_err(|e| format!("set mtime: {e}"))
}

fn value(host: &ModHost) -> Result<i64, String> {
    host.state()
        .get::<Option<i64>>("value")
        .map_err(|e| format!("state.value: {e}"))?
        .ok_or_else(|| "mod never set state.value".to_string())
}

fn scenario(mod_path: &Path, image: &[u8]) -> Result<(), String> {
    write_mod(mod_path, GOOD_V1, 0)?;
    let mut host = ModHost::new(image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    host.load(mod_path).map_err(|e| format!("load: {e}"))?;
    let state = host.state().clone();
    host.advance(&state).map_err(|e| format!("advance: {e}"))?;
    let before = value(&host)?;
    if before != 1 {
        return Err(format!("v1 on_tick left value={before}, want 1"));
    }

    write_mod(mod_path, GOOD_V2, 1)?;
    match host.reload() {
        Ok(true) => println!("reload-check reload ok staged=true sleep=none"),
        other => return Err(format!("reload did not stage the edit: {other:?}")),
    }
    host.advance(&state).map_err(|e| format!("advance after reload: {e}"))?;
    let after = value(&host)?;
    if after != 2 {
        return Err(format!("value on the next tick is {after}, want 2"));
    }
    println!("reload-check value-changed ok from={before} to={after}");

    let version = host.version();
    write_mod(mod_path, BROKEN, 2)?;
    let err = match host.reload() {
        Err(e) => e,
        Ok(staged) => return Err(format!("broken mod was accepted (staged={staged})")),
    };
    if err.file.as_path() != mod_path {
        return Err(format!("ReloadError names {}, want {}", err.file.display(), mod_path.display()));
    }
    if err.message.is_empty() {
        return Err("ReloadError carries an empty message".to_string());
    }
    let line = err.line.ok_or_else(|| format!("ReloadError has no line: {err}"))?;
    println!("reload-error {err}");
    println!("reload-check error-reported line={line} ok");

    host.advance(&state).map_err(|e| format!("advance after failed reload: {e}"))?;
    let kept = value(&host)?;
    if kept != 2 || host.version() != version {
        return Err(format!("rollback left value={kept} version={}, want value=2 version={version}", host.version()));
    }
    println!("reload-check rollback ok value={kept} version={version}");

    let (units, regions, timers) = (host.unit_count(), host.region_names(), host.timer_count());
    write_mod(mod_path, SIDE_EFFECTS_THEN_ERROR, 3)?;
    let err = match host.reload() {
        Err(e) => e,
        Ok(staged) => return Err(format!("side-effect mod was accepted (staged={staged})")),
    };
    if !err.message.contains("boom") {
        return Err(format!("side-effect mod failed for another reason: {err}"));
    }
    let (after_units, after_regions, after_timers) = (host.unit_count(), host.region_names(), host.timer_count());
    if after_units != units || after_regions != regions || after_timers != timers {
        return Err(format!(
            "rejected reload left units={after_units} regions={after_regions:?} timers={after_timers}, want units={units} regions={regions:?} timers={timers}"
        ));
    }
    println!("reload-check rejected reload left world unchanged ok");
    Ok(())
}

fn run() -> Result<(), String> {
    let dir = std::env::temp_dir().join(format!("oscraft-reload-harness-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data");
    let image = osdt_data::generate(&data).map_err(|e| format!("image build: {e:#}"))?;
    let result = scenario(&dir.join("reload.lua"), &image);
    let _ = fs::remove_dir_all(&dir);
    result
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            println!("reload-check harness FAIL {e}");
            ExitCode::FAILURE
        }
    }
}
