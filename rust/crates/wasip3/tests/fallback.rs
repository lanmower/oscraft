//! The driver on the real component: a guest that traps (a Rust panic in the guest) or runs out of fuel (a Rust loop the
//! Lua step budget never sees) ends its run, and the run without mods completes. A mod that loops in Lua is contained
//! by the mod host and never reaches the fuel budget. Needs the fault-injection feature, which builds the guest with
//! the `-- wasip3-fault:` markers and is the feature the driver is built with too:
//!   cd rust/crates/wasip3 && cargo test --features fault-injection --no-fail-fast
#![cfg(feature = "fault-injection")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const IMAGE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../scenarios/moba/moba.osdt");
const MODS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../mods");
const MOD: &str = "structure_buff_example";

// The fault-injection guest, built into the test's own target directory.
fn fault_guest() -> PathBuf {
    let target = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fault-guest");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["build", "--release", "--target", "wasm32-wasip2", "--features", "fault-injection", "--bin", "oscraft_wasip3", "--target-dir"])
        .arg(&target)
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the fault-injection guest builds (needs the wasm32-wasip2 target)");
    target.join("wasm32-wasip2/release/oscraft_wasip3.wasm")
}

// A scratch pack: a copy of the repo's mods with the source of `name` replaced by `source`.
fn pack_with(tag: &str, name: &str, source: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pack-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("pack dir");
    for entry in std::fs::read_dir(MODS).expect("mods dir") {
        let path = entry.expect("mod entry").path();
        if path.extension().is_some_and(|e| e == "lua") {
            std::fs::copy(&path, dir.join(path.file_name().expect("file name"))).expect("copy mod");
        }
    }
    std::fs::write(dir.join(format!("{name}.lua")), source).expect("write mod");
    dir
}

// The repo's source of `name` with a first line that the fault-injection guest reads as a fault.
fn faulty(name: &str, marker: &str) -> String {
    let original = std::fs::read_to_string(Path::new(MODS).join(format!("{name}.lua"))).expect("repo mod");
    format!("{marker}\n{original}")
}

// The driver, run as the test runs it: the guest, the moba image as the sim image and as the mod image, the pack.
fn drive(guest: &Path, pack: &Path) -> (Output, Duration) {
    let start = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_oscraft_wasip3"))
        .arg(guest)
        .arg(IMAGE)
        .arg(IMAGE)
        .env("OSCRAFT_WASIP3_MODS", pack)
        .output()
        .expect("driver runs");
    (output, start.elapsed())
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_guest_trap_ends_the_run_and_the_run_without_mods_completes() {
    let guest = fault_guest();
    let pack = pack_with("panic", MOD, &faulty(MOD, "-- wasip3-fault: panic"));
    let (output, _) = drive(&guest, &pack);
    let out = stdout(&output);
    assert!(output.status.success(), "exit 0 after the fallback: {out}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(out.contains("ev=guest_failed reason=trap:"), "{out}");
    assert!(out.contains("ev=mods_disabled"), "{out}");
    assert!(out.contains("wasip3 tick ok ticks=10"), "{out}");
    assert!(!out.contains("wasip3 mod-trigger ok"), "the mod did not run in the fallback: {out}");
}

#[test]
fn a_runaway_guest_loop_runs_out_of_fuel_and_the_run_without_mods_completes() {
    let guest = fault_guest();
    let pack = pack_with("spin", MOD, &faulty(MOD, "-- wasip3-fault: spin"));
    let (output, elapsed) = drive(&guest, &pack);
    let out = stdout(&output);
    assert!(output.status.success(), "exit 0 after the fallback: {out}\n{}", String::from_utf8_lossy(&output.stderr));
    assert!(out.contains("ev=guest_failed reason=fuel_exhausted"), "{out}");
    assert!(out.contains("wasip3 tick ok ticks=10"), "{out}");
    // The bound only shows the run ends. A release driver takes about 8 s here (measured); a debug driver is several times slower.
    assert!(elapsed < Duration::from_secs(300), "a runaway guest is bounded by the fuel budget, took {elapsed:?}");
}

#[test]
fn a_lua_infinite_loop_is_contained_by_the_mod_host_and_never_reaches_the_fuel_budget() {
    let guest = fault_guest();
    let pack = pack_with("lua-spin", MOD, "return { on_tick = function(state) while true do end end }\n");
    let (output, _) = drive(&guest, &pack);
    let out = stdout(&output);
    assert!(out.contains("ev=mod_hook_failed module=structure_buff_example hook=on_tick"), "the hook is switched off: {out}");
    assert!(out.contains("instruction_limit"), "the Lua step budget ended it: {out}");
    assert!(!out.contains("ev=guest_failed"), "no guest failure: the guest never ran out of fuel: {out}");
    // The example mod's trigger no longer fires, so the driver's own witness check fails the run (exit 1). That is a
    // witness failure, not a guest failure, and it does not fall back.
    assert!(String::from_utf8_lossy(&output.stderr).contains("trigger did not fire"), "{output:?}");
}
