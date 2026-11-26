use std::cell::{Ref, RefMut};
use std::path::Path;

use sim::Sim;

use crate::ModHost;

pub trait ModRuntime {
    fn set_sim(&mut self, sim: Sim);
    fn load_source(&mut self, name: &str, src: &str) -> Result<(), String>;
    fn load_mods(&mut self, mods: &[(String, String)]) -> Result<(), String>;
    fn load_file(&mut self, path: &Path) -> Result<(), String>;
    fn reload(&mut self) -> Result<bool, String>;
    fn load_state(&mut self, bytes: &[u8]) -> Result<(), String>;
    fn advance(&mut self) -> Result<(), String>;
    fn frame(&mut self, dt_ms: i64) -> Result<(), String>;
    fn pointer(&mut self, kind: &str, x: f64, y: f64, button: i64) -> Result<(), String>;
    fn dispatch_action(&mut self, name: &str, args: &serde_json::Value) -> Result<(), String>;
    fn cast_allowed(&mut self, unit: usize, slot: usize, target: i32, x: i32, y: i32, item: bool) -> bool;
    fn cmd_spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize>;
    fn fire_trigger(&mut self, unit: usize, name: &str) -> bool;
    fn player_joined(&mut self, player: usize) -> bool;
    fn player_left(&mut self, player: usize) -> bool;
    fn take_ui(&mut self) -> Vec<String>;
    fn flush_ui(&mut self);
    fn state_line(&mut self) -> String;
    fn state_int(&self, key: &str) -> Option<i64>;
    fn state_bool(&self, key: &str) -> Option<bool>;
    fn units(&self, player: Option<u8>) -> Vec<usize>;
    fn unit_kind(&self, unit: usize) -> Option<String>;
    fn players(&self) -> Vec<usize>;
    fn hook_errors(&self) -> &[String];
    fn disabled_hooks(&self) -> Vec<String>;
    fn version(&self) -> u64;
    fn counters(&self) -> (u64, u64);
    fn events(&self) -> &'static [(&'static str, &'static str)];
    fn sim(&self) -> Ref<'_, Sim>;
    fn sim_mut(&self) -> RefMut<'_, Sim>;
}

impl ModRuntime for ModHost {
    fn set_sim(&mut self, sim: Sim) {
        ModHost::set_sim(self, sim)
    }
    fn load_source(&mut self, name: &str, src: &str) -> Result<(), String> {
        ModHost::load_source(self, name, src)
    }
    fn load_mods(&mut self, mods: &[(String, String)]) -> Result<(), String> {
        ModHost::load_mods(self, mods)
    }
    fn load_file(&mut self, path: &Path) -> Result<(), String> {
        ModHost::load_file(self, path)
    }
    fn reload(&mut self) -> Result<bool, String> {
        ModHost::reload(self)
    }
    fn load_state(&mut self, bytes: &[u8]) -> Result<(), String> {
        ModHost::load_state(self, bytes)
    }
    fn advance(&mut self) -> Result<(), String> {
        ModHost::advance(self)
    }
    fn frame(&mut self, dt_ms: i64) -> Result<(), String> {
        ModHost::frame(self, dt_ms)
    }
    fn pointer(&mut self, kind: &str, x: f64, y: f64, button: i64) -> Result<(), String> {
        ModHost::pointer(self, kind, x, y, button)
    }
    fn dispatch_action(&mut self, name: &str, args: &serde_json::Value) -> Result<(), String> {
        ModHost::dispatch_action(self, name, args)
    }
    fn cast_allowed(&mut self, unit: usize, slot: usize, target: i32, x: i32, y: i32, item: bool) -> bool {
        ModHost::cast_allowed(self, unit, slot, target, x, y, item)
    }
    fn cmd_spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        ModHost::cmd_spawn(self, kind, x, y)
    }
    fn fire_trigger(&mut self, unit: usize, name: &str) -> bool {
        ModHost::fire_trigger(self, unit, name)
    }
    fn player_joined(&mut self, player: usize) -> bool {
        ModHost::player_joined(self, player)
    }
    fn player_left(&mut self, player: usize) -> bool {
        ModHost::player_left(self, player)
    }
    fn take_ui(&mut self) -> Vec<String> {
        ModHost::take_ui(self)
    }
    fn flush_ui(&mut self) {
        ModHost::flush_ui(self)
    }
    fn state_line(&mut self) -> String {
        ModHost::state_line(self)
    }
    fn state_int(&self, key: &str) -> Option<i64> {
        ModHost::state_int(self, key)
    }
    fn state_bool(&self, key: &str) -> Option<bool> {
        ModHost::state_bool(self, key)
    }
    fn units(&self, player: Option<u8>) -> Vec<usize> {
        ModHost::units(self, player)
    }
    fn unit_kind(&self, unit: usize) -> Option<String> {
        ModHost::unit_kind(self, unit)
    }
    fn players(&self) -> Vec<usize> {
        ModHost::players(self)
    }
    fn hook_errors(&self) -> &[String] {
        ModHost::hook_errors(self)
    }
    fn disabled_hooks(&self) -> Vec<String> {
        ModHost::disabled_hooks(self)
    }
    fn version(&self) -> u64 {
        ModHost::version(self)
    }
    fn counters(&self) -> (u64, u64) {
        ModHost::counters(self)
    }
    fn events(&self) -> &'static [(&'static str, &'static str)] {
        ModHost::events(self)
    }
    fn sim(&self) -> Ref<'_, Sim> {
        ModHost::sim(self)
    }
    fn sim_mut(&self) -> RefMut<'_, Sim> {
        ModHost::sim_mut(self)
    }
}
