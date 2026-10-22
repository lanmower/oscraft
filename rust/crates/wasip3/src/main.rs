#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip3_guest::run(stdin.lock(), stdout.lock())
}

// The driver reports a failure as one stderr line and exits non-zero; it does not panic on a guest's failure.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    match cli() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("oscraft_wasip3: {reason}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn cli() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().ok_or("usage: oscraft_wasip3 <guest.wasm> [image.osdt [mod-image.osdt]]")?;
    let bytes = std::fs::read(path).map_err(|e| format!("read guest {path}: {e}"))?;
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(format!("guest {path} is not a wasm binary"));
    }
    println!("wasip3 build ok bytes={}", bytes.len());
    match args.get(1) {
        Some(image) => host::run(path, image, args.get(2).map(String::as_str)),
        None => Ok(()),
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use wasmtime::component::{Component, Linker, ResourceTable};
    use wasmtime::{Config, Engine, Store};
    use wasmtime_wasi::p2::bindings::Command;
    use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
    use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

    /// The commands the native host sends after the image, in the mods-on run. Each reply is one stdout line.
    const COMMANDS: &[&str] = &["tick 10", "ticks", "unit-count", "mod-load"];
    /// How many of COMMANDS are the sim's own (tick, ticks, unit-count); the rest load the test mod.
    const SIM_COMMANDS: usize = 3;
    const MOD_LOADED: &str = "ok mod-load wasip3-test";

    /// The wasm fuel one guest exchange may burn (wasmtime charges about one unit per wasm instruction), so a runaway loop
    /// in the guest ends the exchange as fuel_exhausted instead of holding the driver. Measured on the moba image with the
    /// example mods (wasip2 release guest): a 300-step mod exchange used 7.5e8, the whole 4-exchange run 2.1e9. A Rust
    /// loop burned 1e11 in about 66 s (about 1.5e9 per second), so 1e11 was far too long a freeze. 1e10 is about 13x the
    /// largest measured exchange and a runaway ends in about 7 s. A legitimate exchange over it fails with fuel_exhausted
    /// and falls back to the run without mods; this is the number to raise.
    const GUEST_FUEL: u64 = 10_000_000_000;

    struct State {
        ctx: WasiCtx,
        table: ResourceTable,
    }

    impl WasiView for State {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
        }
    }

    /// The mods directory the host reads from on every run, so an edited mod file is the mod the next
    /// run loads. The guest has no file access: its source arrives as a `mod-source` command.
    const MODS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../mods");

    /// The mods whose state the host prints after 300 steps on the scenario image, as the native
    /// mod_check prints it: one `mod-state` line each.
    const STATE_MODS: &[&str] = &["waves_example", "custom_spell"];

    /// Why a scenario run failed. A guest that traps or runs out of fuel is a `Guest` failure: the driver runs the
    /// scenario again with the pack's mods disabled. Any other failure is a `Driver` failure and ends the driver.
    enum Failure {
        Guest(String),
        Driver(String),
    }

    impl From<String> for Failure {
        fn from(e: String) -> Self {
            Failure::Driver(e)
        }
    }

    impl Failure {
        fn message(self) -> String {
            match self {
                Failure::Guest(reason) => format!("guest run: {reason}"),
                Failure::Driver(e) => e,
            }
        }
    }

    /// The `mod-source` command that hands the guest the current text of mods/<name>.lua. OSCRAFT_WASIP3_MODS names
    /// another directory of mods, for the tests that run a scratch pack.
    fn load_command(name: &str) -> Result<String, String> {
        let dir = std::env::var("OSCRAFT_WASIP3_MODS").unwrap_or_else(|_| MODS_DIR.to_owned());
        let path = format!("{dir}/{name}.lua");
        let src = std::fs::read(&path).map_err(|e| format!("read mod {path}: {e}"))?;
        let hex: String = src.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!("mod-source {name} {hex}"))
    }

    fn send(guest: &str, image_path: &str, commands: &[String]) -> Result<Vec<String>, Failure> {
        let refs: Vec<&str> = commands.iter().map(String::as_str).collect();
        exchange(guest, image_path, &refs)
    }

    /// Runs the guest on the image, checks the tick and mod-load witness, then runs the example mod script on
    /// `mod_image` when one is given. A guest trap or fuel exhaustion ends that run as `ev=guest_failed`, and the same
    /// scenario runs again with the pack's mods disabled (the last-known-good run); the driver exits non-zero only when
    /// that run fails too. With no failure this is one run, as before.
    pub fn run(guest: &str, image_path: &str, mod_image: Option<&str>) -> Result<(), String> {
        for line in with_fallback(|mods, report| scenario(guest, image_path, mod_image, mods, report))? {
            println!("{line}");
        }
        Ok(())
    }

    /// One attempt with the mods; a guest failure ends it as `ev=guest_failed` and the attempt runs again with the mods
    /// disabled. A driver failure is not retried. Returns the success lines of the attempt that completed.
    fn with_fallback(mut attempt: impl FnMut(bool, &mut Vec<String>) -> Result<(), Failure>) -> Result<Vec<String>, String> {
        let mut report = Vec::new();
        match attempt(true, &mut report) {
            Ok(()) => return Ok(report),
            Err(Failure::Driver(e)) => return Err(e),
            Err(Failure::Guest(reason)) => {
                println!("ev=guest_failed reason={reason}");
                println!("ev=mods_disabled");
            }
        }
        let mut report = Vec::new();
        attempt(false, &mut report).map(|()| report).map_err(Failure::message)
    }

    /// One pass of the scenario. `mods` false is the fallback: the sim runs alone, with no mod loaded and no mod run.
    /// The success lines go to `report`, printed by the caller only when the pass completes.
    fn scenario(guest: &str, image_path: &str, mod_image: Option<&str>, mods: bool, report: &mut Vec<String>) -> Result<(), Failure> {
        let commands = if mods { COMMANDS } else { &COMMANDS[..SIM_COMMANDS] };
        let replies = exchange(guest, image_path, commands)?;
        if replies.len() != commands.len() || replies[..SIM_COMMANDS].iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected replies: {replies:?}").into());
        }
        let ticks = replies[1].strip_prefix("ok ticks ").ok_or_else(|| format!("ticks reply {:?}", replies[1]))?;
        let units = replies[2].strip_prefix("ok unit-count ").ok_or_else(|| format!("unit-count reply {:?}", replies[2]))?;
        report.push(format!("wasip3 tick ok ticks={ticks} units={units}"));
        if !mods {
            report.push("wasip3 mods disabled: no mod loaded, no mod run".to_owned());
            return Ok(());
        }
        if replies[3] != MOD_LOADED {
            return Err(format!("mod-load reply {:?}, want {MOD_LOADED:?}", replies[3]).into());
        }
        report.push("wasip3 mod-load ok mod=wasip3-test".to_owned());
        let Some(mod_image) = mod_image else { return Ok(()) };
        let commands = [load_command("structure_buff_example")?, "mod-step".into(), "mod-kill tower".into(), "mod-step".into()];
        let replies = send(guest, mod_image, &commands)?;
        if replies.len() != commands.len() || replies.iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected mod replies: {replies:?}").into());
        }
        let (fired, applied) = counters(&replies[3])?;
        if fired == 0 || applied == 0 {
            return Err(format!("trigger did not fire or no command applied: {:?}", replies[3]).into());
        }
        report.push(format!("wasip3 mod-trigger ok mod=structure_buff_example fired={fired} applied={applied}"));
        for name in STATE_MODS {
            let commands = [load_command(name)?, "mod-step 300".to_string(), "mod-state".to_string()];
            let replies = send(guest, mod_image, &commands)?;
            if replies.len() != commands.len() || replies.iter().any(|r| r.starts_with("err")) {
                return Err(format!("unexpected replies for {name}: {replies:?}").into());
            }
            let line = replies[2].strip_prefix("ok ").ok_or_else(|| format!("mod-state reply {:?}", replies[2]))?;
            report.push(line.to_owned());
        }
        Ok(())
    }

    fn counters(reply: &str) -> Result<(u64, u64), String> {
        let rest = reply.strip_prefix("ok mod-step fired=").ok_or_else(|| format!("mod-step reply {reply:?}"))?;
        let (fired, applied) = rest.split_once(" applied=").ok_or_else(|| format!("mod-step reply {reply:?}"))?;
        Ok((fired.parse().map_err(|e| format!("fired: {e}"))?, applied.parse().map_err(|e| format!("applied: {e}"))?))
    }

    /// The engine every guest runs on: the async component model, with fuel metered so a run can be bounded.
    fn engine() -> Result<Engine, String> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        config.consume_fuel(true);
        Engine::new(&config).map_err(|e| format!("engine: {e:#}"))
    }

    /// The one-line reason a guest call failed. Fuel exhaustion and a trap are named (a guest panic is a trap, since the
    /// wasm build aborts on panic); any other error prints as it came.
    fn guest_reason(e: &wasmtime::Error) -> String {
        match e.downcast_ref::<wasmtime::Trap>() {
            Some(wasmtime::Trap::OutOfFuel) => format!("fuel_exhausted: the guest used its {GUEST_FUEL} fuel budget"),
            Some(trap) => format!("trap: {trap}"),
            None => format!("{e:#}"),
        }
    }

    /// Runs the guest component on the image at `image_path`, sends `commands` over stdin, and returns the reply lines.
    /// The guest's `ev=` lines (a mod switched off, a refused command) are not replies: the driver prints them as they come.
    fn exchange(guest: &str, image_path: &str, commands: &[&str]) -> Result<Vec<String>, Failure> {
        let image = std::fs::read(image_path).map_err(|e| format!("read image {image_path}: {e}"))?;
        let mut input = (image.len() as u32).to_le_bytes().to_vec();
        input.extend_from_slice(&image);
        for line in commands {
            input.extend_from_slice(line.as_bytes());
            input.push(b'\n');
        }
        let stdout = MemoryOutputPipe::new(1 << 20);
        let engine = engine()?;
        let component = Component::from_file(&engine, guest).map_err(|e| format!("component {guest}: {e:#}"))?;
        // The guest is a wasi:cli/run@0.2 component (wasm32-wasip2); the p2 linker provides its imports.
        let mut linker = Linker::<State>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(|e| format!("linker: {e:#}"))?;
        let mut builder = WasiCtx::builder();
        builder.stdin(MemoryInputPipe::new(input)).stdout(stdout.clone());
        let mut store = Store::new(&engine, State { ctx: builder.build(), table: ResourceTable::default() });
        store.set_fuel(GUEST_FUEL).map_err(|e| format!("fuel: {e:#}"))?;
        let runtime = tokio::runtime::Runtime::new().map_err(|e| format!("tokio: {e}"))?;
        let result: wasmtime::Result<Result<(), ()>> = runtime.block_on(async {
            let command = Command::instantiate_async(&mut store, &component, &linker).await?;
            command.wasi_cli_run().call_run(&mut store).await
        });
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(e) if e.downcast_ref::<wasmtime::Trap>().is_some() => return Err(Failure::Guest(guest_reason(&e))),
            Err(e) => return Err(Failure::Driver(format!("guest run: {e:#}"))),
        };
        outcome.map_err(|()| Failure::Driver("guest exited with an error".to_owned()))?;
        let out = String::from_utf8(stdout.contents().to_vec()).map_err(|e| format!("stdout utf8: {e}"))?;
        let mut replies = Vec::new();
        for line in out.lines() {
            if line.starts_with("ev=") {
                println!("{line}");
            } else {
                replies.push(line.to_owned());
            }
        }
        Ok(replies)
    }

    #[cfg(test)]
    mod tests {
        use super::{engine, guest_reason, with_fallback, Failure};
        use wasmtime::{Instance, Module, Store};

        // Runs `with_fallback` with one scripted outcome per attempt; returns its result and the mods flag of each attempt.
        fn attempts(outcomes: Vec<Result<(), Failure>>) -> (Result<Vec<String>, String>, Vec<bool>) {
            let mut seen = Vec::new();
            let mut outcomes = outcomes.into_iter();
            let result = with_fallback(|mods, report| {
                seen.push(mods);
                report.push(format!("pass mods={mods}"));
                outcomes.next().expect("no more attempts than outcomes")
            });
            (result, seen)
        }

        #[test]
        fn a_run_without_failure_is_one_attempt_with_mods() {
            let (result, seen) = attempts(vec![Ok(())]);
            assert_eq!(result, Ok(vec!["pass mods=true".to_owned()]));
            assert_eq!(seen, [true]);
        }

        #[test]
        fn a_guest_failure_runs_the_scenario_again_without_mods() {
            let (result, seen) = attempts(vec![Err(Failure::Guest("trap: x".into())), Ok(())]);
            assert_eq!(result, Ok(vec!["pass mods=false".to_owned()]));
            assert_eq!(seen, [true, false]);
        }

        #[test]
        fn a_fallback_that_also_fails_is_an_error() {
            let (result, seen) = attempts(vec![Err(Failure::Guest("trap: x".into())), Err(Failure::Guest("fuel: y".into()))]);
            assert_eq!(result, Err("guest run: fuel: y".to_owned()));
            assert_eq!(seen, [true, false]);
        }

        #[test]
        fn a_driver_failure_is_not_retried_without_mods() {
            let (result, seen) = attempts(vec![Err(Failure::Driver("unexpected replies".into()))]);
            assert_eq!(result, Err("unexpected replies".to_owned()));
            assert_eq!(seen, [true]);
        }

        // Calls export `name` of the module `wat` with `fuel` units; the failure as the driver reports it.
        fn call(wat: &str, name: &str, fuel: u64) -> Result<(), String> {
            let engine = engine()?;
            let module = Module::new(&engine, wat).map_err(|e| format!("module: {e:#}"))?;
            let mut store = Store::new(&engine, ());
            store.set_fuel(fuel).map_err(|e| format!("fuel: {e:#}"))?;
            let instance = Instance::new(&mut store, &module, &[]).map_err(|e| guest_reason(&e))?;
            let func = instance.get_typed_func::<(), ()>(&mut store, name).map_err(|e| format!("export: {e:#}"))?;
            func.call(&mut store, ()).map_err(|e| guest_reason(&e))
        }

        #[test]
        fn a_runaway_guest_loop_runs_out_of_fuel_and_is_an_error() {
            let spin = r#"(module (func (export "spin") (loop $l (br $l))))"#;
            let reason = call(spin, "spin", 1_000_000).expect_err("a loop that never returns must not return");
            assert!(reason.starts_with("fuel_exhausted"), "{reason}");
        }

        #[test]
        fn a_guest_trap_is_an_error_naming_the_trap() {
            let boom = r#"(module (func (export "boom") unreachable))"#;
            let reason = call(boom, "boom", 1_000_000).expect_err("a trap must be an error");
            assert!(reason.starts_with("trap:"), "{reason}");
        }
    }
}
