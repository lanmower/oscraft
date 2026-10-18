//! Browser entry point. `Game` loads a map image (the bytes sim::load reads) and steps the sim;
//! the page draws the units it reports.

use wasm_bindgen::prelude::*;

/// The unit cap the gate uses, so the browser and the parity checks load the same limits.
const MAX_UNITS: usize = 1024;

#[wasm_bindgen]
pub struct Game {
    sim: sim::Sim,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(image: &[u8]) -> Result<Game, JsValue> {
        let sim = sim::load(image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        let sim = sim;
        Ok(Game { sim })
    }

    pub fn tick(&mut self) {
        self.sim.tick();
    }

    pub fn ticks(&self) -> u32 {
        self.sim.ticks()
    }

    pub fn state_hash(&self) -> u64 {
        self.sim.state_hash()
    }

    /// The live state as sim save bytes, for the page to keep across a host restart. The sim is only read.
    pub fn save_state(&self) -> Vec<u8> {
        self.sim.save_state()
    }

    /// Restores the sim from bytes `save_state` wrote on the same map. A refused load leaves the game as it was.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.sim.load_state(bytes).map_err(|e| JsValue::from_str(&format!("load_state: {e:?}")))
    }

    /// Sim ids of the living units, in the same order as `units()`.
    pub fn unit_ids(&self) -> Vec<u32> {
        (0..self.sim.count())
            .filter(|&i| self.sim.alive(i))
            .map(|i| i as u32)
            .collect()
    }

    /// 1 when the living unit is mid-strike (its weapon is recharging after a hit), else 0, in the same order as `unit_ids()`.
    pub fn unit_attacking(&self) -> Vec<u8> {
        (0..self.sim.count())
            .filter(|&i| self.sim.alive(i))
            .map(|i| u8::from(self.sim.weapon_cooldown(i) > 0))
            .collect()
    }

    /// Hit points of the living units, in whole points, in the same order as `unit_ids()`.
    pub fn unit_hps(&self) -> Vec<i32> {
        (0..self.sim.count())
            .filter(|&i| self.sim.alive(i))
            .map(|i| sim::fixed::to_int(self.sim.hp(i)))
            .collect()
    }

    /// Orders unit `unit` to walk to tile (x, y). Returns false when the sim refuses the order.
    pub fn order_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        // sim's ORDER_MOVE (movement order kind 0); the sim keeps that constant crate-private.
        const ORDER_MOVE: u8 = 0;
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.sim.issue_order_kind(unit as usize, ORDER_MOVE, fx, fy).is_ok()
    }

    /// Hit points of unit `unit`, in whole points.
    pub fn unit_hp(&self, unit: u32) -> i32 {
        sim::fixed::to_int(self.sim.hp(unit as usize))
    }

    /// Gold (resource a) of player `player`, in whole units.
    pub fn gold(&self, player: u8) -> i32 {
        self.sim.resource_a(player)
    }

    /// The winning side, or -1 while the match runs.
    pub fn winner(&self) -> i32 {
        self.sim.winner().map_or(-1, |side| side as i32)
    }

    /// Post-game screen data as JSON: `{"winner":side|-1,"players":[{"player","kills","deaths","assists","gold","xp","items":[kind,...]}]}`.
    pub fn post_game_json(&self) -> String {
        let players: Vec<String> = self
            .sim
            .post_game()
            .iter()
            .map(|t| {
                let items: Vec<String> = t.items.iter().map(|k| k.to_string()).collect();
                format!(
                    "{{\"player\":{},\"kills\":{},\"deaths\":{},\"assists\":{},\"gold\":{},\"xp\":{},\"items\":[{}]}}",
                    t.player,
                    t.kills,
                    t.deaths,
                    t.assists,
                    t.gold,
                    t.xp,
                    items.join(",")
                )
            })
            .collect();
        format!("{{\"winner\":{},\"players\":[{}]}}", self.winner(), players.join(","))
    }

    /// Casts ability slot `slot` of unit `unit` at tile (x, y), aimed at unit `target` or at the ground when
    /// `target` is -1. Returns false when the sim refuses the cast.
    pub fn cast(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.sim.cast_ability(unit as usize, slot as usize, target, fx, fy)
    }

    /// Orders unit `unit` to attack unit `target`, walking toward it when it is out of reach.
    /// Returns false when the sim refuses the order.
    pub fn attack(&mut self, unit: u32, target: u32) -> bool {
        self.sim.order_attack_or_approach(unit as usize, target as usize).is_ok()
    }

    /// Test hook for the page's CI probe: deals `damage` whole points to unit `unit`.
    pub fn hurt(&mut self, unit: u32, damage: i32) {
        self.sim.hurt(unit as usize, sim::fixed::from_int(damage));
    }

    /// Kills credited to unit `unit`.
    pub fn kills_of(&self, unit: u32) -> u32 {
        self.sim.kills_of(unit as usize)
    }

    /// Deaths unit `unit` has suffered.
    pub fn deaths_of(&self, unit: u32) -> u32 {
        self.sim.deaths_of(unit as usize)
    }

    /// Spawns a hero of unit type `kind` for `player` at tile (x, y). Returns its unit id, or -1 when refused.
    pub fn spawn_hero(&mut self, player: u8, kind: u32, x: f32, y: f32) -> i32 {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.sim.spawn_hero(player, kind, fx, fy).map_or(-1, |i| i as i32)
    }

    /// Loads the hero rules from `key=value` text (scenarios/moba/hero_rules.txt). False, rules unchanged, on a bad line.
    pub fn set_hero_rules(&mut self, text: &str) -> bool {
        self.sim.set_hero_rules_text(text)
    }

    /// Ticks until hero `unit` respawns; 0 when alive; -1 for a non-hero.
    pub fn respawn_ticks_left(&self, unit: u32) -> i32 {
        self.sim.hero_respawn_left(unit as usize)
    }

    /// Gold a buyback of hero `unit` costs now; -1 for a non-hero.
    pub fn buyback_cost(&self, unit: u32) -> i32 {
        self.sim.hero_buyback_cost(unit as usize)
    }

    /// Ticks until hero `unit` may buy back again; 0 when ready; -1 for a non-hero.
    pub fn buyback_cooldown_ticks(&self, unit: u32) -> i32 {
        self.sim.hero_buyback_cooldown_left(unit as usize)
    }

    /// Buys back dead hero `unit` at its home. False when refused: alive, on cooldown, short of gold, or home cell full.
    pub fn buyback(&mut self, unit: u32) -> bool {
        self.sim.buyback(unit as usize)
    }

    /// Gold held by hero `unit`; 0 for a non-hero.
    pub fn hero_gold(&self, unit: u32) -> i32 {
        self.sim.hero_gold(unit as usize)
    }

    /// Ticks left on ability `slot` of unit `unit` (0 when ready); -1 for an unknown unit or slot.
    pub fn ability_cooldown(&self, unit: u32, slot: u32) -> i32 {
        self.sim.ability_cooldown(unit as usize, slot as usize)
    }

    /// Kill events since the last drain, flat as `[tick, killer, killer_player, victim, victim_player]` per event.
    pub fn drain_kill_events(&mut self) -> Vec<u32> {
        self.sim
            .drain_kill_events()
            .into_iter()
            .flat_map(|e| [e.tick, e.killer, e.killer_player as u32, e.victim, e.victim_player as u32])
            .collect()
    }

    /// Turns the fog of war on or off. The moba preview turns it off so every enemy is a valid target.
    pub fn set_fog(&mut self, on: bool) {
        self.sim.set_fog(on);
    }

    /// Living units as flat `[x, y, player]` triples, positions in tiles.
    pub fn units(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..self.sim.count() {
            if !self.sim.alive(i) {
                continue;
            }
            let (x, y) = self.sim.position_fixed(i);
            out.push(sim::fixed::to_float(x) as f32);
            out.push(sim::fixed::to_float(y) as f32);
            out.push(self.sim.player(i) as f32);
        }
        out
    }

    /// Living units' type index, one per unit in `units()` order: the unit declaration order.
    pub fn unit_types(&self) -> Vec<u32> {
        (0..self.sim.count())
            .filter(|&i| self.sim.alive(i))
            .map(|i| self.sim.unit_type(i).max(0) as u32)
            .collect()
    }

    /// The model file each catalog unit type draws with, indexed by type index. Empty when none.
    pub fn unit_model_names(&self) -> Vec<String> {
        let cat = self.sim.catalog();
        cat.units.iter().map(|u| cat.name(u.model).to_owned()).collect()
    }
}
