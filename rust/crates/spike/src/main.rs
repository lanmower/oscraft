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
  spawn_unit("skirmisher", 20 * 65536, 20 * 65536)
end
"#;

const V3_BAD_SYNTAX: &str = r#"function on_tick(state)
  state.ticks = state.ticks + 1
  spawn_unit("skirmisher", 20 * 65536, 20 * 65536
end
"#;

const V4_RUNTIME_ERR: &str = r#"function on_tick(state)
  state.ticks = state.ticks + 1
  local t = nil
  return t.x
end
"#;

fn run_2000(image: &[u8], path: &Path) -> Result<(u64, usize), Box<dyn Error>> {
    let mut h = ModHost::new(image)?;
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
    let spike_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_mod = spike_dir.join("mods/unit_mod.lua");

    // The unit catalog is the .osd sources the data crate reads (rust/data). The
    // sim's catalog gives the spawn index space, so one image is built per run.
    let image = osdt_data::generate(&spike_dir.join("../../data")).map_err(|e| e.to_string())?;

    let (a, units) = run_2000(&image, &repo_mod)?;
    let (b, _) = run_2000(&image, &repo_mod)?;
    println!("determinism: 2000 ticks, {units} units");
    println!("determinism: run1 fnv1a={a:016x} run2 fnv1a={b:016x} match={}", a == b);
    if a != b {
        return Err("determinism mismatch".into());
    }

    let mut hooked = ModHost::new(&image)?;
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
    let mut host = ModHost::new(&image)?;
    host.load(&live)?;
    for _ in 0..5 {
        host.reload_if_changed();
        host.tick(host.state())?;
    }
    write_version(&live, V2_RELOAD, 1)?;
    host.reload_if_changed();
    host.tick(host.state())?;
    let before_bad = host.unit_count();
    write_version(&live, V3_BAD_SYNTAX, 2)?;
    host.reload_if_changed();
    host.tick(host.state())?;
    let kept = host.unit_count() == before_bad + 1;
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
