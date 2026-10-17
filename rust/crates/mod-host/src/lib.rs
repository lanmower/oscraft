//! Lua mod host. A mod is a chunk that defines `on_tick(state)`. The host owns `state` and the
//! sim; a mod reload never resets them. The native host runs mods on mlua (Lua 5.4, C objects);
//! the wasip3 guest runs them on piccolo (pure Rust), a reduced surface in piccolo_host.rs.

#[cfg(all(feature = "mlua", feature = "piccolo"))]
compile_error!("mod-host: the mlua and piccolo features are exclusive");

const MAX_UNITS: usize = 1024;

// The OSDT image for a directory of `.osd` files, from the generate path `osdt-data <dir> <out>` runs.
// Behind the `build` feature so a guest that only loads images never links the starlark toolchain.
#[cfg(feature = "build")]
pub fn build_image(src_dir: &std::path::Path) -> Result<Vec<u8>, String> {
    osdt_data::generate(src_dir).map_err(|e| format!("{e:#}"))
}

#[cfg(feature = "mlua")]
mod mlua_host;
#[cfg(feature = "mlua")]
pub use mlua_host::*;

#[cfg(feature = "piccolo")]
mod piccolo_host;
#[cfg(feature = "piccolo")]
pub use piccolo_host::*;
