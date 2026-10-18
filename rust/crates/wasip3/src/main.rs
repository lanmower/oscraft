#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip3_guest::run(stdin.lock(), stdout.lock())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
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

    /// The mods whose state the host prints after 300 steps on the moba image, as the native
    /// mod_check prints it: one `mod-state` line each.
    const STATE_MODS: &[&str] = &["td_waves", "custom_spell"];

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
        let commands = [load_command("tower_buff")?, "mod-step".into(), "mod-kill tower".into(), "mod-step".into()];
        let replies = send(guest, mod_image, &commands)?;
        if replies.len() != commands.len() || replies.iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected mod replies: {replies:?}"));
        }
        let (fired, applied) = counters(&replies[3])?;
        if fired == 0 || applied == 0 {
            return Err(format!("trigger did not fire or no command applied: {:?}", replies[3]));
        }
        println!("wasip3 mod-trigger ok mod=tower_buff fired={fired} applied={applied}");
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
        let mut config = Config::new();
        config.wasm_component_model_async(true);
        let engine = Engine::new(&config).map_err(|e| format!("engine: {e:#}"))?;
        let component = Component::from_file(&engine, guest).map_err(|e| format!("component {guest}: {e:#}"))?;
        let mut linker = Linker::<State>::new(&engine);
        wasmtime_wasi::p3::add_to_linker(&mut linker).map_err(|e| format!("linker: {e:#}"))?;
        // The built guest imports wasi:io/error@0.2.12; the p3 linker does not provide that instance.
        wasmtime_wasi::p2::add_to_linker_async(&mut linker).map_err(|e| format!("linker p2: {e:#}"))?;
        let mut builder = WasiCtx::builder();
        builder.stdin(MemoryInputPipe::new(input)).stdout(stdout.clone());
        let mut store = Store::new(&engine, State { ctx: builder.build(), table: ResourceTable::default() });
        let runtime = tokio::runtime::Runtime::new().map_err(|e| format!("tokio: {e}"))?;
        let result = runtime.block_on(async {
            let command = Command::instantiate_async(&mut store, &component, &linker).await?;
            store.run_concurrent(async move |store| command.wasi_cli_run().call_run(store).await).await?
        });
        result.map_err(|e| format!("guest run: {e:#}"))?.map_err(|()| "guest exited with an error".to_string())?;
        let out = String::from_utf8(stdout.contents().to_vec()).map_err(|e| format!("stdout utf8: {e}"))?;
        Ok(out.lines().map(str::to_owned).collect())
    }
}
