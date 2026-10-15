use mlua::{HookTriggers, VmState};
use mod_host::ModHost;
use std::cell::Cell;
use std::error::Error;
use std::fs::{self, File};
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

const V2_RELOAD: &str = r#"function on_tick(state)
  state.ticks = state.ticks + 1
  spawn_unit("wraith", state.ticks * 65536, 0)
end
"#;

const V3_BAD_SYNTAX: &str = r#"function on_tick(state)
  state.ticks = state.ticks + 1
  spawn_unit("wraith", state.ticks * 65536, 0
end
"#;

const V4_RUNTIME_ERR: &str = r#"function on_tick(state)
  state.ticks = state.ticks + 1
  local t = nil
  return t.x
end
"#;

fn run_2000(path: &Path) -> Result<(u64, usize), Box<dyn Error>> {
    let mut h = ModHost::new();
    h.load(path)?;
    for _ in 0..2000 {
        h.reload_if_changed();
        h.tick(h.state())?;
    }
    Ok((h.state_hash(), h.unit_count()))
}

// Writes a mod version and moves its mtime forward explicitly, so the poll
// sees a change without sleeping.
fn write_version(path: &Path, src: &str, n: u64) -> std::io::Result<()> {
    fs::write(path, src)?;
    let when = SystemTime::now() + Duration::from_secs(2 * n);
    File::options().write(true).open(path)?.set_modified(when)
}

fn main() -> Result<(), Box<dyn Error>> {
    let repo_mod = Path::new(env!("CARGO_MANIFEST_DIR")).join("mods/unit_mod.lua");

    let (a, units) = run_2000(&repo_mod)?;
    let (b, _) = run_2000(&repo_mod)?;
    println!("determinism: 2000 ticks, {units} units");
    println!("determinism: run1 fnv1a={a:016x} run2 fnv1a={b:016x} match={}", a == b);
    if a != b {
        return Err("determinism mismatch".into());
    }

    let mut hooked = ModHost::new();
    hooked.load(&repo_mod)?;
    let lines = Rc::new(Cell::new(0u64));
    let counter = lines.clone();
    hooked.lua().set_hook(HookTriggers::EVERY_LINE, move |_, _| {
        counter.set(counter.get() + 1);
        Ok(VmState::Continue)
    });
    hooked.reload_if_changed();
    hooked.tick(hooked.state())?;
    hooked.lua().remove_hook();
    println!("hook: on_tick executed {} lines in one tick", lines.get());

    let live = std::env::temp_dir().join("oscraft_spike_unit_mod.lua");
    fs::copy(&repo_mod, &live)?;
    let mut host = ModHost::new();
    host.load(&live)?;
    for _ in 0..5 {
        host.reload_if_changed();
        host.tick(host.state())?;
    }
    write_version(&live, V2_RELOAD, 1)?;
    host.reload_if_changed();
    host.tick(host.state())?;
    let before_bad = host.count_kind("wraith");
    write_version(&live, V3_BAD_SYNTAX, 2)?;
    host.reload_if_changed();
    host.tick(host.state())?;
    let kept = host.count_kind("wraith") == before_bad + 1;
    println!("reload: bad reload kept old functions: {}", if kept { "yes" } else { "no" });

    write_version(&live, V4_RUNTIME_ERR, 3)?;
    for _ in 0..2 {
        host.reload_if_changed();
        if let Err(e) = host.tick(host.state()) {
            println!("tick error, loop continues: {e}");
        }
    }
    println!("state.ticks across reloads and errors: {}", host.state().get::<i64>("ticks")?);
    Ok(())
}
