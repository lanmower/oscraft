#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip3_guest::run(stdin.lock(), stdout.lock())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().ok_or("usage: oscraft_wasip3 <guest.wasm> [image.osdt]")?;
    let bytes = std::fs::read(path).map_err(|e| format!("read guest {path}: {e}"))?;
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(format!("guest {path} is not a wasm binary"));
    }
    println!("wasip3 build ok bytes={}", bytes.len());
    match args.get(1) {
        Some(image) => host::run(path, image),
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
    const COMMANDS: &[&str] = &["tick 10", "ticks", "unit-count", "mod-load probe"];
    const MOD_REFUSAL: &str = "this guest is built without the mod runtime";

    struct State {
        ctx: WasiCtx,
        table: ResourceTable,
    }

    impl WasiView for State {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView { ctx: &mut self.ctx, table: &mut self.table }
        }
    }

    /// Runs the guest component on the image, sends the commands over stdin, and checks the replies.
    pub fn run(guest: &str, image_path: &str) -> Result<(), String> {
        let image = std::fs::read(image_path).map_err(|e| format!("read image {image_path}: {e}"))?;
        let mut input = (image.len() as u32).to_le_bytes().to_vec();
        input.extend_from_slice(&image);
        for line in COMMANDS {
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
        let replies: Vec<&str> = out.lines().collect();
        if replies.len() != COMMANDS.len() || replies[..3].iter().any(|r| r.starts_with("err")) {
            return Err(format!("unexpected replies: {replies:?}"));
        }
        let ticks = replies[1].strip_prefix("ok ticks ").ok_or_else(|| format!("ticks reply {:?}", replies[1]))?;
        let units = replies[2].strip_prefix("ok unit-count ").ok_or_else(|| format!("unit-count reply {:?}", replies[2]))?;
        println!("wasip3 tick ok ticks={ticks} units={units}");
        let refusal = replies[3].strip_prefix("err ").ok_or_else(|| format!("mod-load reply {:?}", replies[3]))?;
        if refusal != MOD_REFUSAL {
            return Err(format!("mod-load not refused: {refusal}"));
        }
        println!("wasip3 mod-load refused reason=no mod runtime");
        Ok(())
    }
}
