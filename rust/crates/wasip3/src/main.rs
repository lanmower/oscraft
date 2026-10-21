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
    use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
    use wasmtime_wasi::p3::bindings::Command;
    use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

    /// The commands the native host sends after the image. Each reply is one stdout line.
    const COMMANDS: &[&str] = &["tick 10", "ticks", "unit-count", "mod-load"];
    const MOD_LOADED: &str = "ok mod-load wasip3-test";

    /// The fuel one guest exchange may burn (wasmtime charges about one unit per wasm instruction). The guest runs under
    /// it, so a runaway loop in the guest ends the exchange as a named error instead of holding the driver forever.
    /// Not measured: the component could not be built where this was written. 1e11 is a guess with a wide margin over a
    /// 300-step scenario run; a legitimate exchange that trips it fails loudly with fuel_exhausted, and this is the number to raise.
    const GUEST_FUEL: u64 = 100_000_000_000;

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

    /// The `mod-source` command that hands the guest the current text of mods/<name>.lua.
    fn load_command(name: &str) -> Result<String, String> {
        let path = format!("{MODS_DIR}/{name}.lua");
        let src = std::fs::read(&path).map_err(|e| format!("read mod {path}: {e}"))?;
        let hex: String = src.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!("mod-source {name} {hex}"))
    }

    fn send(guest: &str, image_path: &str, commands: &[String]) -> Result<Vec<String>, String> {
        let refs: Vec<&str> = commands.iter().map(String::as_str).collect();
        exchange(guest, image_path, &refs)
    }

    /// Runs the guest on the image, checks the tick and mod-load witness, then runs the example
    /// mod script on `mod_image` when one is given.
    pub fn run(guest: &str, image_path: &str, mod_image: Option<&str>) -> Result<(), String> {
        let replies = exchange(guest, image_path, COMMANDS)?;
        if replies.len() != COMMANDS.len() || replies[..3].iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected replies: {replies:?}"));
        }
        let ticks = replies[1].strip_prefix("ok ticks ").ok_or_else(|| format!("ticks reply {:?}", replies[1]))?;
        let units = replies[2].strip_prefix("ok unit-count ").ok_or_else(|| format!("unit-count reply {:?}", replies[2]))?;
        println!("wasip3 tick ok ticks={ticks} units={units}");
        if replies[3] != MOD_LOADED {
            return Err(format!("mod-load reply {:?}, want {MOD_LOADED:?}", replies[3]));
        }
        println!("wasip3 mod-load ok mod=wasip3-test");
        let Some(mod_image) = mod_image else { return Ok(()) };
        let commands = [load_command("structure_buff_example")?, "mod-step".into(), "mod-kill tower".into(), "mod-step".into()];
        let replies = send(guest, mod_image, &commands)?;
        if replies.len() != commands.len() || replies.iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected mod replies: {replies:?}"));
        }
        let (fired, applied) = counters(&replies[3])?;
        if fired == 0 || applied == 0 {
            return Err(format!("trigger did not fire or no command applied: {:?}", replies[3]));
        }
        println!("wasip3 mod-trigger ok mod=structure_buff_example fired={fired} applied={applied}");
        for name in STATE_MODS {
            let commands = [load_command(name)?, "mod-step 300".to_string(), "mod-state".to_string()];
            let replies = send(guest, mod_image, &commands)?;
            if replies.len() != commands.len() || replies.iter().any(|r| r.starts_with("err")) {
                return Err(format!("unexpected replies for {name}: {replies:?}"));
            }
            let line = replies[2].strip_prefix("ok ").ok_or_else(|| format!("mod-state reply {:?}", replies[2]))?;
            println!("{line}");
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

    /// Runs the guest component on the image at `image_path`, sends `commands` over stdin, and
    /// returns the reply lines.
    fn exchange(guest: &str, image_path: &str, commands: &[&str]) -> Result<Vec<String>, String> {
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
        wasmtime_wasi::p3::add_to_linker(&mut linker).map_err(|e| format!("linker: {e:#}"))?;
        // The built guest imports wasi:io/error@0.2.12; the p3 linker does not provide that instance.
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(|e| format!("linker p2: {e:#}"))?;
        let mut builder = WasiCtx::builder();
        builder.stdin(MemoryInputPipe::new(input)).stdout(stdout.clone());
        let mut store = Store::new(&engine, State { ctx: builder.build(), table: ResourceTable::default() });
        store.set_fuel(GUEST_FUEL).map_err(|e| format!("fuel: {e:#}"))?;
        let runtime = tokio::runtime::Runtime::new().map_err(|e| format!("tokio: {e}"))?;
        let result = runtime.block_on(async {
            let command = Command::instantiate_async(&mut store, &component, &linker).await?;
            store.run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await).await?
        });
        let outcome = result.map_err(|e| format!("guest run: {}", guest_reason(&e)))?;
        outcome.map_err(|()| "guest exited with an error".to_string())?;
        let out = String::from_utf8(stdout.contents().to_vec()).map_err(|e| format!("stdout utf8: {e}"))?;
        Ok(out.lines().map(str::to_owned).collect())
    }

    #[cfg(test)]
    mod tests {
        use super::{engine, guest_reason};
        use wasmtime::{Instance, Module, Store};

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
