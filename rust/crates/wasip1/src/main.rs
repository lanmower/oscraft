#[cfg(target_arch = "wasm32")]
fn main() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    wasip1_guest::run(stdin.lock(), stdout.lock())
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use std::path::Path;

    use anyhow::{anyhow, bail, Context, Result};
    use wasmtime::{Engine, Linker, Module, Store};
    use wasmtime_wasi::pipe::{MemoryInputPipe, MemoryOutputPipe};
    use wasmtime_wasi::preview1::{self, WasiP1Ctx};
    use wasmtime_wasi::WasiCtxBuilder;

    const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
    const DEFAULT_GUEST: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/target/wasm32-wasip1/release/oscraft_wasip1.wasm"
    );
    const TICKS: u32 = 10;

    pub fn main() -> Result<()> {
        let path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| DEFAULT_GUEST.to_string());
        let image = osdt_data::generate(Path::new(DATA_DIR)).context("build the OSDT image from rust/data")?;
        let mut input = (image.len() as u32).to_le_bytes().to_vec();
        input.extend_from_slice(&image);
        input.extend_from_slice(format!("tick {TICKS}\nstate\n").as_bytes());

        let engine = Engine::default();
        let module = Module::from_file(&engine, &path).with_context(|| format!("load guest {path}"))?;
        let mut linker: Linker<WasiP1Ctx> = Linker::new(&engine);
        preview1::add_to_linker_sync(&mut linker, |ctx| ctx)?;
        let stdout = MemoryOutputPipe::new(1 << 20);
        let mut builder = WasiCtxBuilder::new();
        builder.stdin(MemoryInputPipe::new(input)).stdout(stdout.clone());
        let mut store = Store::new(&engine, builder.build_p1());
        let instance = linker.instantiate(&mut store, &module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;
        start.call(&mut store, ())?;

        let text = String::from_utf8(stdout.contents().to_vec())?;
        let ticked = format!("ok tick {TICKS}");
        if !text.lines().any(|line| line == ticked) {
            bail!("guest did not acknowledge tick {TICKS}: {text}");
        }
        let state = text
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("state "))
            .ok_or_else(|| anyhow!("no state reply: {text}"))?;
        let units: u32 = state
            .split_whitespace()
            .find_map(|kv| kv.strip_prefix("units="))
            .ok_or_else(|| anyhow!("no units in state reply: {state}"))?
            .parse()?;
        if units == 0 {
            bail!("state reply reports zero units: {state}");
        }
        println!("wasip1 tick ok ticks={TICKS} units={units}");
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    host::main()
}
