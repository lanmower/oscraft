#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip3_guest::run(stdin.lock(), stdout.lock())
}

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

    const COMMANDS: &[&str] = &["tick 10", "ticks", "unit-count", "mod-load"];
    const SIM_COMMANDS: usize = 3;
    const MOD_LOADED: &str = "ok mod-load wasip3-test";

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

    const MODS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../mods");

    const STATE_MODS: &[&str] = &["waves_example", "custom_spell"];

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

    pub fn run(guest: &str, image_path: &str, mod_image: Option<&str>) -> Result<(), String> {
        for line in with_fallback(|mods, report| scenario(guest, image_path, mod_image, mods, report))? {
            println!("{line}");
        }
        Ok(())
    }

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

    fn engine() -> Result<Engine, String> {
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        config.consume_fuel(true);
        Engine::new(&config).map_err(|e| format!("engine: {e:#}"))
    }

    fn guest_reason(e: &wasmtime::Error) -> String {
        match e.downcast_ref::<wasmtime::Trap>() {
            Some(wasmtime::Trap::OutOfFuel) => format!("fuel_exhausted: the guest used its {GUEST_FUEL} fuel budget"),
            Some(trap) => format!("trap: {trap}"),
            None => format!("{e:#}"),
        }
    }

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
