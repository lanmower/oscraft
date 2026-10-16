//! Native host: instantiates the wasip2 sim component, loads the generated image and runs one
//! tick. With the `mod` feature it drives the mod host natively instead of through the component:
//! it loads a test mod whose binding spawns a unit beside unit 0, checks that the state hash moves, fires a
//! trigger and reads the mod version.
//! Usage: oscraft-wasip2-host [component.wasm]

use std::path::Path;

use anyhow::{anyhow, Context, Result};

const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");

#[cfg(not(feature = "mod"))]
const MAX_UNITS: u32 = 1024;
#[cfg(not(feature = "mod"))]
const DEFAULT_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/target/wasm32-wasip2/release/oscraft_wasip2.wasm"
);

#[cfg(feature = "mod")]
const TEST_MOD_NAME: &str = "wasip2-test-mod.lua";
#[cfg(feature = "mod")]
const TRIGGER: &str = "idle";

#[cfg(not(feature = "mod"))]
mod component {
    use anyhow::{anyhow, Result};
    use wasmtime::component::{Component, Linker, ResourceTable};
    use wasmtime::{Engine, Store};
    use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

    pub struct Host {
        wasi: WasiCtx,
        table: ResourceTable,
    }

    impl WasiView for Host {
        fn ctx(&mut self) -> &mut WasiCtx {
            &mut self.wasi
        }

        fn table(&mut self) -> &mut ResourceTable {
            &mut self.table
        }
    }

    pub fn run(path: &str, image: Vec<u8>, max_units: u32) -> Result<()> {
        use anyhow::Context;

        let engine = Engine::default();
        let component =
            Component::from_file(&engine, path).with_context(|| format!("load component {path}"))?;
        let mut linker: Linker<Host> = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        let host = Host {
            wasi: WasiCtxBuilder::new().build(),
            table: ResourceTable::new(),
        };
        let mut store = Store::new(&engine, host);
        let instance = linker.instantiate(&mut store, &component)?;

        let load = instance.get_typed_func::<(Vec<u8>, u32), (Result<(), String>,)>(&mut store, "load")?;
        let (loaded,) = load.call(&mut store, (image, max_units))?;
        load.post_return(&mut store)?;
        loaded.map_err(|e| anyhow!("load: {e}"))?;

        let tick = instance.get_typed_func::<(), ()>(&mut store, "tick")?;
        tick.call(&mut store, ())?;
        tick.post_return(&mut store)?;

        let ticks = instance.get_typed_func::<(), (u32,)>(&mut store, "ticks")?;
        let (ticked,) = ticks.call(&mut store, ())?;
        ticks.post_return(&mut store)?;

        let units = instance.get_typed_func::<(), (u32,)>(&mut store, "unit-count")?;
        let (count,) = units.call(&mut store, ())?;
        units.post_return(&mut store)?;

        println!("wasip2 tick ok ticks={ticked} units={count}");
        Ok(())
    }
}

#[cfg(not(feature = "mod"))]
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_COMPONENT.to_string());
    let image = osdt_data::generate(Path::new(DATA_DIR)).context("build the OSDT image from rust/data")?;
    component::run(&path, image, MAX_UNITS)
}

#[cfg(feature = "mod")]
fn main() -> Result<()> {
    use mod_host::ModHost;

    let image = osdt_data::generate(Path::new(DATA_DIR)).context("build the OSDT image from rust/data")?;
    let load = || ModHost::new(&image).map_err(|e| anyhow!("load image: {e:?}"));

    let mut plain = load()?;
    let plain_state = plain.state().clone();
    plain.advance(&plain_state).map_err(|e| anyhow!("advance without a mod: {e}"))?;
    let plain_hash = plain.state_hash();
    let (unit_name, x, y) = {
        let sim = plain.sim();
        if sim.count() == 0 {
            return Err(anyhow!("the image has no units to spawn beside"));
        }
        let catalog = sim.catalog();
        let unit = catalog.units.first().ok_or_else(|| anyhow!("the catalog has no units"))?;
        let (x, y) = sim.position_fixed(0);
        (catalog.name(unit.name).to_string(), x, y)
    };

    let mut modded = load()?;
    let source = format!("function on_tick(state) spawn_unit({unit_name:?}, {x}, {y}) end\n");
    modded.load_source(TEST_MOD_NAME, &source).map_err(|e| anyhow!("mod-load: {e}"))?;
    let modded_state = modded.state().clone();
    modded.advance(&modded_state).map_err(|e| anyhow!("advance with the mod: {e}"))?;
    if modded.state_hash() == plain_hash {
        return Err(anyhow!("mod hash unchanged: spawn_unit did not change the sim state"));
    }
    println!("wasip2 mod hash changed");

    let fired = modded.sim().fire_trigger(0, TRIGGER);
    println!("wasip2 trigger ok fired={fired}");
    println!("wasip2 mod ok version={}", modded.version());
    if !fired {
        return Err(anyhow!("trigger: idle did not fire on unit 0"));
    }
    if modded.version() < 1 {
        return Err(anyhow!("mod: version {} after load, expected at least 1", modded.version()));
    }
    let unit0_kind = modded.sim().kind_of(0);
    if unit0_kind == 0 {
        return Err(anyhow!("catalog: unit 0 has kind 0"));
    }
    println!("wasip2 catalog ok unit0_kind={unit0_kind}");
    Ok(())
}
