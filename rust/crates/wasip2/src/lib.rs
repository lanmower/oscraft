//! The sim as a wasip2 component. The WIT world is wit/sim-component.wit.
//! Native builds compile this crate empty; the host binary is src/host.rs.
//! The `mod` feature links the mod host (mlua, vendored Lua) into the component. Without it the
//! component runs the sim alone and mod-load reports that no mod runtime is built in.

#[cfg(target_arch = "wasm32")]
mod guest {
    use std::cell::RefCell;

    #[cfg(feature = "mod")]
    use mod_host::ModHost as Backend;
    #[cfg(not(feature = "mod"))]
    use plain::Plain as Backend;

    wit_bindgen::generate!({
        world: "sim-component",
        path: "wit",
    });

    #[cfg(not(feature = "mod"))]
    mod plain {
        use std::cell::{RefCell, RefMut};

        use sim::{LoadError, Sim};

        const MAX_UNITS: usize = 1024;

        pub struct Plain(RefCell<Sim>);

        impl Plain {
            pub fn new(image: &[u8]) -> Result<Self, LoadError> {
                Ok(Plain(RefCell::new(sim::load(image, MAX_UNITS)?)))
            }

            pub fn sim(&self) -> RefMut<'_, Sim> {
                self.0.borrow_mut()
            }

            pub fn advance(&mut self) {
                self.0.borrow_mut().tick();
            }

            pub fn unit_count(&self) -> usize {
                self.0.borrow().count()
            }

            pub fn state_hash(&self) -> u64 {
                self.0.borrow().state_hash()
            }
        }
    }

    struct Component;

    thread_local! {
        static HOST: RefCell<Option<Backend>> = const { RefCell::new(None) };
    }

    fn with_host<T>(absent: T, f: impl FnOnce(&mut Backend) -> T) -> T {
        HOST.with(|slot| match slot.borrow_mut().as_mut() {
            Some(host) => f(host),
            None => absent,
        })
    }

    impl Guest for Component {
        fn load(image: Vec<u8>, _max_units: u32) -> Result<(), String> {
            let host = Backend::new(&image).map_err(|e| format!("{e:?}"))?;
            HOST.with(|slot| *slot.borrow_mut() = Some(host));
            Ok(())
        }

        #[cfg(feature = "mod")]
        fn mod_load(name: String, src: String) -> Result<(), String> {
            with_host(Err("no sim loaded".to_string()), |h| h.load_source(&name, &src).map_err(|e| e.to_string()))
        }

        #[cfg(not(feature = "mod"))]
        fn mod_load(_name: String, _src: String) -> Result<(), String> {
            Err("this component is built without the mod runtime".to_string())
        }

        #[cfg(feature = "mod")]
        fn mod_version() -> u64 {
            with_host(0, |h| h.version())
        }

        #[cfg(not(feature = "mod"))]
        fn mod_version() -> u64 {
            0
        }

        #[cfg(feature = "mod")]
        fn tick() {
            with_host((), |h| {
                let state = h.state().clone();
                h.advance(&state).expect("advance the mod host by one tick")
            })
        }

        #[cfg(not(feature = "mod"))]
        fn tick() {
            with_host((), |h| h.advance())
        }

        fn ticks() -> u32 {
            with_host(0, |h| h.sim().ticks())
        }

        fn unit_count() -> u32 {
            with_host(0, |h| h.unit_count() as u32)
        }

        fn alive_count() -> u32 {
            with_host(0, |h| h.sim().alive_count() as u32)
        }

        fn state_hash() -> u64 {
            with_host(0, |h| h.state_hash())
        }

        fn unit_position(unit: u32) -> (i32, i32) {
            with_host((0, 0), |h| h.sim().position_fixed(unit as usize))
        }

        fn unit_player(unit: u32) -> u8 {
            with_host(0, |h| h.sim().player(unit as usize))
        }

        fn unit_hp(unit: u32) -> i32 {
            with_host(0, |h| h.sim().hp(unit as usize))
        }

        fn order(unit: u32, x: i32, y: i32) {
            with_host((), |h| h.sim().order(unit as usize, x, y))
        }

        fn order_all(player: u8, x: i32, y: i32) {
            with_host((), |h| h.sim().order_all(player, x, y))
        }

        fn fire_trigger(unit: u32, name: String) -> bool {
            with_host(false, |h| h.sim().fire_trigger(unit as usize, &name))
        }

        fn catalog_unit_name(kind: u32) -> Option<String> {
            with_host(None, |h| {
                let sim = h.sim();
                let catalog = sim.catalog();
                catalog.units.get(kind as usize).map(|u| catalog.name(u.name).to_string())
            })
        }
    }

    export!(Component);
}