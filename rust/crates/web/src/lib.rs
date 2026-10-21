//! Browser entry point. `Game` loads a map image (the bytes sim::load reads) and steps the sim;
//! the page draws the units it reports.

use wasm_bindgen::prelude::*;

// The replay format and player live in the replay crate, which records the files this plays.
use replay::{Command, Recording, Replayer};

// The mod host: the pack's Lua runs on piccolo over the sim this Game steps.
use mod_host::ModHost;

/// The image name the live recording runs on. The browser loads map bytes, so it has no file name to record.
const LIVE_IMAGE: &str = "map";

/// The unit cap the gate uses, so the browser and the parity checks load the same limits.
const MAX_UNITS: usize = 1024;

#[wasm_bindgen]
pub struct Game {
    /// The mod host owns the live sim; mods read and command it through its game.* bindings.
    host: ModHost,
    image: Vec<u8>,
    replay: Option<Replayer>,
    /// The commands the live sim received, so a data swap re-bases the record the same way the server's does.
    rec: Recording,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(image: &[u8]) -> Result<Game, JsValue> {
        let host = ModHost::new(image).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        let rec = Recording::new(LIVE_IMAGE, MAX_UNITS, &host.sim());
        Ok(Game { host, image: image.to_vec(), replay: None, rec })
    }

    /// Replaces the sim with a fresh load of the map image and cues the replay `text` (the file
    /// the server's GET /replay serves) at tick 0. Returns the replay's last tick. Refused when the
    /// replay's initial state hash differs from this map's, since it was recorded on other data.
    pub fn replay_load(&mut self, text: &str) -> Result<u32, JsValue> {
        let rec = Recording::parse(text).map_err(replay_error)?;
        let fresh = sim::load(&self.image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        let player = Replayer::new(rec, &fresh).map_err(replay_error)?;
        let end = player.end();
        self.rec.mark_incomplete(fresh.ticks());
        self.host.set_sim(fresh);
        self.replay = Some(player);
        Ok(end)
    }

    /// Advances the loaded replay by up to `n` ticks, checking each recorded hash on the way.
    /// Stops at the end of the replay. Returns the tick reached.
    pub fn replay_step(&mut self, n: u32) -> Result<u32, JsValue> {
        let player = self.replay.as_mut().ok_or_else(|| JsValue::from_str("no replay loaded"))?;
        for _ in 0..n {
            if self.host.sim().ticks() >= player.end() {
                break;
            }
            player.step(&mut self.host.sim_mut()).map_err(replay_error)?;
        }
        Ok(self.host.sim().ticks())
    }

    /// Moves the loaded replay to `tick`. Going back re-simulates from tick 0 on a fresh load.
    pub fn replay_seek(&mut self, tick: u32) -> Result<u32, JsValue> {
        let player = self.replay.as_mut().ok_or_else(|| JsValue::from_str("no replay loaded"))?;
        let tick = tick.min(player.end());
        if tick < self.host.sim().ticks() {
            let fresh = sim::load(&self.image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
            player.reset(&fresh).map_err(replay_error)?;
            self.rec.mark_incomplete(fresh.ticks());
            self.host.set_sim(fresh);
        }
        player.seek(&mut self.host.sim_mut(), tick).map_err(replay_error)
    }

    /// Last tick of the loaded replay; 0 when none is loaded.
    pub fn replay_end(&self) -> u32 {
        self.replay.as_ref().map_or(0, Replayer::end)
    }

    /// Tick from which the loaded replay may diverge (a data reload or an unrecordable mod command), or -1.
    pub fn replay_incomplete(&self) -> i32 {
        self.replay.as_ref().and_then(|p| p.recording().incomplete()).map_or(-1, |t| t as i32)
    }

    /// Map size in tiles as `[width, height]`.
    pub fn map_size(&self) -> Vec<u32> {
        let world = self.host.sim();
        let g = world.grid();
        vec![g.w as u32, g.h as u32]
    }

    /// Advances the match one tick. With mods loaded, each mod's on_tick runs in the step (the mod host's order).
    /// Returns the mod error text for the page to log, or "" when the step ran clean. A failed step leaves the
    /// sim as the step left it; the next tick runs again.
    pub fn tick(&mut self) -> String {
        if self.host.version() == 0 {
            self.rec.tick(&mut self.host.sim_mut(), true, true);
            return String::new();
        }
        self.host.advance().err().unwrap_or_default()
    }

    /// Loads the pack's mods from `json`, an array of {"name", "text"} objects in the manifest's order. Each mod is
    /// its own Lua environment over this match's sim. Every mod compiles before any takes effect: a failing mod keeps
    /// the last-known-good set and the error text is returned for the page to log. Returns "" when the set loaded.
    /// A replay of a match with mods is not reproducible, so the record is marked from this tick on.
    pub fn load_mods(&mut self, json: &str) -> String {
        let mods = match parse_mods(json) {
            Ok(mods) => mods,
            Err(e) => return e,
        };
        match self.host.load_mods(&mods) {
            Ok(()) => {
                self.rec.mark_incomplete(self.host.sim().ticks());
                String::new()
            }
            Err(e) => e,
        }
    }

    /// Runs the pack's on_action(name, args, state). `args` is a JSON value ("" for none). Returns the error text, or "".
    pub fn dispatch_action(&mut self, name: &str, args: &str) -> String {
        let args: serde_json::Value = match serde_json::from_str(if args.trim().is_empty() { "null" } else { args }) {
            Ok(v) => v,
            Err(e) => return format!("dispatch_action {name}: bad args JSON: {e}"),
        };
        self.host.dispatch_action(name, &args).err().unwrap_or_default()
    }

    /// Runs the pack's on_pointer(kind, x, y, button, state). `kind` is "down", "up" or "move"; x and y are tile
    /// coordinates. Returns the error text, or "".
    pub fn pointer(&mut self, kind: &str, x: f64, y: f64, button: i32) -> String {
        self.host.pointer(kind, x, y, i64::from(button)).err().unwrap_or_default()
    }

    /// Swaps the live match onto the map image `image` (new data). The live units carry over onto it, as on the server,
    /// and the record starts a new segment at this tick. Returns `[carried, refused]`: the live units carried, and the
    /// live units the new data had no slot for. Refused when the image does not load; the match is unchanged then.
    pub fn swap_data(&mut self, image: &[u8]) -> Result<Vec<u32>, JsValue> {
        let mut fresh = sim::load(image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        fresh.set_fog(self.host.sim().fog_on());
        let (carried, refused) = sim::carry_from(&self.host.sim(), &mut fresh);
        self.rec.rebase(LIVE_IMAGE, MAX_UNITS, &fresh);
        self.host.set_sim(fresh);
        self.image = image.to_vec();
        Ok(vec![carried as u32, refused as u32])
    }

    pub fn ticks(&self) -> u32 {
        self.host.sim().ticks()
    }

    pub fn state_hash(&self) -> u64 {
        self.host.sim().state_hash()
    }

    /// The live state as sim save bytes, for the page to keep across a host restart. The sim is only read.
    pub fn save_state(&self) -> Vec<u8> {
        self.host.sim().save_state()
    }

    /// Restores the sim from bytes `save_state` wrote on the same map. A refused load leaves the game as it was.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.host.sim_mut().load_state(bytes).map_err(|e| JsValue::from_str(&format!("load_state: {e:?}")))?;
        self.rec.mark_incomplete(self.host.sim().ticks());
        Ok(())
    }

    /// Sim ids of the living units, in the same order as `units()`.
    pub fn unit_ids(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| i as u32)
            .collect()
    }

    /// 1 when the living unit is mid-strike (its weapon is recharging after a hit), else 0, in the same order as `unit_ids()`.
    pub fn unit_attacking(&self) -> Vec<u8> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| u8::from(self.host.sim().weapon_cooldown(i) > 0))
            .collect()
    }

    /// Hit points of the living units, in whole points, in the same order as `unit_ids()`.
    pub fn unit_hps(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| sim::fixed::to_int(self.host.sim().hp(i)))
            .collect()
    }

    /// Orders unit `unit` to walk to tile (x, y). Returns false when the sim refuses the order.
    pub fn order_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        // sim's ORDER_MOVE (movement order kind 0); the sim keeps that constant crate-private.
        const ORDER_MOVE: u8 = 0;
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.rec.apply(&mut self.host.sim_mut(), Command::Kind { unit: unit as usize, kind: ORDER_MOVE, x: fx, y: fy })
    }

    /// Attack-moves unit `unit` to tile (x, y): it walks there, strikes each enemy in its reach on the way,
    /// and resumes travel after each kill. Returns false when the sim refuses the order.
    pub fn attack_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.rec.apply(&mut self.host.sim_mut(), Command::Kind { unit: unit as usize, kind: sim::ORDER_ATTACK_MOVE, x: fx, y: fy })
    }

    /// Holds unit `unit` where it stands: it does not chase, and strikes only enemies in its reach.
    /// Returns false when the sim refuses the order (dead or unknown unit, stunned).
    pub fn hold(&mut self, unit: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::Hold { unit: unit as usize })
    }

    /// Debug view: the target class a mod set on unit `unit` (0..=3), or -1 when the Rust rule applies.
    pub fn class_override(&self, unit: u32) -> i32 {
        self.host.sim().class_override_of(unit as usize).map_or(-1, i32::from)
    }

    /// Debug view: the attack-move hold a mod set on unit `unit` (1 always holds, 0 never), or -1 when the Rust rule applies.
    pub fn hold_override(&self, unit: u32) -> i32 {
        self.host.sim().hold_override_of(unit as usize).map_or(-1, i32::from)
    }

    /// Sets unit `unit`'s target class to `class` (0..=3), or clears it with -1. The change goes through the replay
    /// command the mods use, so a recording and the peers that replay it see the same change. False when the sim refuses it.
    pub fn set_class_override(&mut self, unit: u32, class: i32) -> bool {
        let class = match class {
            -1 => 255,
            0..=254 => class as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.host.sim_mut(), Command::ClassOverride { unit, class })
    }

    /// Sets unit `unit`'s attack-move hold: 1 always holds, 0 never holds, -1 restores the Rust rule.
    /// Same replay command path as `set_class_override`. False when the sim refuses it.
    pub fn set_hold_override(&mut self, unit: u32, hold: i32) -> bool {
        let hold = match hold {
            -1 => 255,
            0 | 1 => hold as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.host.sim_mut(), Command::HoldOverride { unit, hold })
    }

    /// Name of the sim's most recent refusal (`Refusal` variant), for the page's refusal witness.
    pub fn last_refusal(&self) -> String {
        format!("{:?}", self.host.sim().stats.last_refusal)
    }

    /// Hit points of unit `unit`, in whole points.
    pub fn unit_hp(&self, unit: u32) -> i32 {
        sim::fixed::to_int(self.host.sim().hp(unit as usize))
    }

    /// Gold (resource a) of player `player`, in whole units.
    pub fn gold(&self, player: u8) -> i32 {
        self.host.sim().resource_a(player)
    }

    /// The winning side, or -1 while the match runs.
    pub fn winner(&self) -> i32 {
        self.host.sim().winner().map_or(-1, |side| side as i32)
    }

    /// Post-game screen data as JSON: `{"winner":side|-1,"players":[{"player","kills","deaths","assists","gold","xp","items":[kind,...]}]}`.
    pub fn post_game_json(&self) -> String {
        let players: Vec<String> = self
            .host.sim()
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
        self.rec.apply(&mut self.host.sim_mut(), Command::Cast { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    /// Uses the active item in inventory slot `slot` of unit `unit`, aimed at unit `target` or at the
    /// ground (tile x, y) when `target` is -1. Returns false when the sim refuses it (empty slot, item
    /// cooldown, range); a refused use changes nothing.
    pub fn use_item(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.rec.apply(&mut self.host.sim_mut(), Command::UseItem { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    /// Spends one skill point to raise ability slot `slot` of unit `unit` by a rank. Returns false
    /// when the sim refuses it (no free point, max rank, level gate, or no skill tree).
    pub fn skill_up(&mut self, unit: u32, slot: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::SkillUp { unit: unit as usize, slot: slot as usize })
    }

    /// True when player `a` is an enemy of player `b`: different teams and no alliance.
    pub fn hostile(&self, a: u32, b: u32) -> bool {
        self.host.sim().hostile(a as u8, b as u8)
    }

    /// Orders unit `unit` to attack unit `target`, walking toward it when it is out of reach.
    /// Returns false when the sim refuses the order.
    pub fn attack(&mut self, unit: u32, target: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::Attack { unit: unit as usize, target: target as usize })
    }

    /// Test hook for the page's CI probe: deals `damage` whole points to unit `unit`.
    pub fn hurt(&mut self, unit: u32, damage: i32) {
        self.host.sim_mut().hurt(unit as usize, sim::fixed::from_int(damage));
        self.rec.mark_incomplete(self.host.sim().ticks());
    }

    /// Kills credited to unit `unit`.
    pub fn kills_of(&self, unit: u32) -> u32 {
        self.host.sim().kills_of(unit as usize)
    }

    /// Deaths unit `unit` has suffered.
    pub fn deaths_of(&self, unit: u32) -> u32 {
        self.host.sim().deaths_of(unit as usize)
    }

    /// Spawns a unit of unit type `kind` for `player` at tile (x, y). Returns its unit id, or -1 when refused.
    pub fn spawn_attributed(&mut self, player: u8, kind: u32, x: f32, y: f32) -> i32 {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        let id = self.host.sim_mut().spawn_attributed(player, kind, fx, fy).map_or(-1, |i| i as i32);
        self.rec.mark_incomplete(self.host.sim().ticks());
        id
    }

    /// Removes live unit `unit` from the sim: its slot stops counting and no query finds it. False when it is not alive.
    pub fn remove_unit(&mut self, unit: u32) -> bool {
        let removed = self.host.sim_mut().remove_unit(unit as usize);
        self.rec.mark_incomplete(self.host.sim().ticks());
        removed
    }

    /// Gold held by unit `unit`; 0 for an unknown unit.
    pub fn unit_gold(&self, unit: u32) -> i32 {
        self.host.sim().unit_gold(unit as usize)
    }

    /// Ticks left on ability `slot` of unit `unit` (0 when ready); -1 for an unknown unit or slot.
    pub fn ability_cooldown(&self, unit: u32, slot: u32) -> i32 {
        self.host.sim().ability_cooldown(unit as usize, slot as usize)
    }

    /// Ticks left on a status of `unit`: `kind` 0 stun, 1 slow, 2 silence, 3 root, 4 invisibility
    /// (`sim::STATUS_NAMES`). 0 when the status is off or the unit or kind is unknown.
    pub fn status_left(&self, unit: u32, kind: u32) -> i32 {
        self.host.sim().status_left(unit as usize, kind as usize)
    }

    pub fn stun_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 0)
    }

    pub fn slow_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 1)
    }

    /// The slow's speed multiplier, Q16 (65536 when the unit is not slowed).
    pub fn slow_multiplier(&self, unit: u32) -> i32 {
        self.host.sim().slow_multiplier(unit as usize)
    }

    pub fn silence_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 2)
    }

    pub fn root_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 3)
    }

    pub fn invisible_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 4)
    }

    /// Kill events since the last drain, flat as `[tick, killer, killer_player, victim, victim_player]` per event.
    pub fn drain_kill_events(&mut self) -> Vec<u32> {
        self.host.sim_mut()
            .drain_kill_events()
            .into_iter()
            .flat_map(|e| [e.tick, e.killer, e.killer_player as u32, e.victim, e.victim_player as u32])
            .collect()
    }

    /// UI events the pack queued since the last drain (game.ui_event, camera, message, banner, panel, sound), as a JSON
    /// array of objects: {"kind": ..., ...}. The page renders them and decides nothing.
    pub fn drain_ui(&mut self) -> String {
        let bodies = self.host.take_ui();
        format!("[{}]", bodies.iter().map(|b| format!("{{{b}}}")).collect::<Vec<_>>().join(","))
    }

    /// Display effects queued by mods since the last drain, as a JSON array of objects with kind, x, y (fixed point),
    /// source, target, team, colour, duration, text, scale, name and value. The page draws them and decides nothing.
    pub fn drain_effects(&mut self) -> String {
        let mut out = String::from("[");
        for (i, e) in self.host.sim_mut().take_fx().iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!(
                "{{\"kind\":{},\"x\":{},\"y\":{},\"source\":{},\"target\":{},\"team\":{},\"colour\":{},\"duration\":{},\"text\":{},\"scale\":{},\"name\":{},\"value\":{}}}",
                json_str(&e.kind),
                e.x,
                e.y,
                e.source,
                e.target,
                e.team,
                json_str(&e.colour),
                e.duration,
                json_str(&e.text),
                e.scale,
                json_str(&e.name),
                e.value
            ));
        }
        out.push(']');
        out
    }

    /// Turns the fog of war on or off. The preview turns it off so every enemy is a valid target.
    pub fn set_fog(&mut self, on: bool) {
        self.rec.apply(&mut self.host.sim_mut(), Command::Fog { on });
    }

    // fogview: vision grid [width, height, cell size in tiles] and player `p`'s visible cells, one byte each.
    pub fn vision_dims(&self) -> Vec<u32> {
        let (w, h, s) = self.host.sim().vision_dims();
        vec![w as u32, h as u32, s as u32]
    }

    pub fn vision(&self, p: u8) -> Vec<u8> {
        self.host.sim().vision_plane(p)
    }

    // fogview: changes whenever a visible plane may have changed; equal values mean `vision` would repeat its bytes.
    pub fn vision_generation(&self) -> u32 {
        self.host.sim().vision_generation()
    }

    pub fn fog_on(&self) -> bool {
        self.host.sim().fog_on()
    }

    // fogview: day and night clock as `[ticks into cycle, day ticks, night ticks]`.
    pub fn day_cycle(&self) -> Vec<u32> {
        vec![self.host.sim().day_clock(), sim::DAY_TICKS, sim::NIGHT_TICKS]
    }

    /// Living units as flat `[x, y, player]` triples, positions in tiles.
    pub fn units(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..self.host.sim().count() {
            if !self.host.sim().alive(i) {
                continue;
            }
            let (x, y) = self.host.sim().position_fixed(i);
            out.push(sim::fixed::to_float(x) as f32);
            out.push(sim::fixed::to_float(y) as f32);
            out.push(self.host.sim().player(i) as f32);
        }
        out
    }

    /// Living units' type index, one per unit in `units()` order: the unit declaration order.
    pub fn unit_types(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().unit_type(i).max(0) as u32)
            .collect()
    }

    /// The model file each catalog unit type draws with, indexed by type index. Empty when none.
    pub fn unit_model_names(&self) -> Vec<String> {
        let world = self.host.sim();
        let cat = world.catalog();
        cat.units.iter().map(|u| cat.name(u.model).to_owned()).collect()
    }

    /// The catalog name of each unit type, indexed by type index: the index `spawn_attributed` takes.
    pub fn unit_type_names(&self) -> Vec<String> {
        let world = self.host.sim();
        let cat = world.catalog();
        cat.units.iter().map(|u| cat.name(u.name).to_owned()).collect()
    }

    /// Per unit type, indexed like unit_type_names: 1 when the type is player-controlled (unit_extra
    /// player_controlled=True), 0 for every other kind. The page reads it as a data flag, not a genre.
    pub fn unit_flags(&self) -> Vec<u8> {
        self.host.sim().catalog().unit_ext.iter().map(|e| e.player_controlled as u8).collect()
    }

    /// Test hook for the page's CI probe: puts `count` charges of catalog item index `item` into unit `unit`'s inventory. False when no slot is free.
    pub fn give_item(&mut self, unit: u32, item: u32, count: u32) -> bool {
        let added = self.host.sim_mut().inventory_add(unit as usize, item + 1, count);
        self.rec.mark_incomplete(self.host.sim().ticks());
        added
    }

    /// UI export: the item catalog as JSON `[{"id":index,"name","cost","armor","damage","health","flags",
    /// "tier","tags","charges","stack","stats":{name:value,...},"components":[index,...],"target","radius"}]`
    /// (`target` and `radius` describe the item's active ability, see `targeting`). `stats` lists
    /// the non-zero bonuses; `tags` is the bit mask of the tags the scenario declared (item_tags), in declaration order.
    pub fn item_catalog_json(&self) -> String {
        let world = self.host.sim();
        let cat = world.catalog();
        let rows: Vec<String> = cat
            .items
            .iter()
            .enumerate()
            .map(|(id, it)| {
                let parts: Vec<String> =
                    it.components.iter().filter(|&&c| c != 0xFFFF).map(|c| c.to_string()).collect();
                let stats: Vec<String> = sim::ITEM_STATS
                    .iter()
                    .zip(it.stats)
                    .filter(|(_, v)| *v != 0)
                    .map(|(n, v)| format!("\"{n}\":{v}"))
                    .collect();
                let (target, radius) = targeting(cat, it.ability as usize);
                format!(
                    "{{\"id\":{},\"name\":{},\"cost\":{},\"armor\":{},\"damage\":{},\"health\":{},\"flags\":{},\"tier\":{},\"tags\":{},\"charges\":{},\"stack\":{},\"stats\":{{{}}},\"components\":[{}],\"target\":\"{}\",\"radius\":{}}}",
                    id,
                    json_str(cat.name(it.name)),
                    it.cost,
                    it.stats[0],
                    it.stats[1],
                    it.stats[2],
                    it.flags,
                    it.tier,
                    it.tags,
                    it.charges,
                    it.stack_max,
                    stats.join(","),
                    parts.join(","),
                    target,
                    radius
                )
            })
            .collect();
        format!("[{}]", rows.join(","))
    }

    /// UI export: unit `unit`'s inventory and stash as JSON `{"slots":[[kind,count],...],"stash":[[kind,count],...]}`;
    /// kind 0 is empty, otherwise the item catalog index plus one.
    pub fn inventory_json(&self, unit: u32) -> String {
        let bank = |read: &dyn Fn(usize) -> (u32, u32)| {
            let cells: Vec<String> = (0..self.host.sim().inventory_slots())
                .map(|s| {
                    let (k, c) = read(s);
                    format!("[{k},{c}]")
                })
                .collect();
            cells.join(",")
        };
        let u = unit as usize;
        format!(
            "{{\"slots\":[{}],\"stash\":[{}]}}",
            bank(&|s| self.host.sim().inventory_slot(u, s)),
            bank(&|s| self.host.sim().stash_slot(u, s))
        )
    }

    /// UI export: the abilities unit `unit` holds as JSON `[{"slot","name","cooldown","energy","ultimate","max_rank",
    /// "gates":[...],"damage":[...],"cooldowns":[...],"energies":[...],"target":"none|unit|point","radius"}]`
    /// (world units, 0 for `none`); ability stats are game ticks and
    /// the skill arrays are empty for an ability with no skill tree.
    pub fn abilities_json(&self, unit: u32) -> String {
        let world = self.host.sim();
        let cat = world.catalog();
        let list = |v: &[i32]| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        let rows: Vec<String> = (0..sim::ABILITY_SLOTS)
            .filter_map(|slot| {
                let ai = self.host.sim().ability_in_slot(unit as usize, slot)?;
                let a = cat.abilities.get(ai)?;
                let (ult, max, g, d, c, e) = match a.skill {
                    Some(s) => {
                        let n = s.max_rank as usize;
                        (s.ultimate, n, list(&s.gates[..n]), list(&s.damage[..n]), list(&s.cooldown[..n]), list(&s.energy[..n]))
                    }
                    None => (false, 0, String::new(), String::new(), String::new(), String::new()),
                };
                let (target, radius) = targeting(cat, ai);
                Some(format!(
                    "{{\"slot\":{},\"name\":{},\"cooldown\":{},\"energy\":{},\"ultimate\":{},\"max_rank\":{},\"gates\":[{}],\"damage\":[{}],\"cooldowns\":[{}],\"energies\":[{}],\"target\":\"{}\",\"radius\":{}}}",
                    slot,
                    json_str(cat.name(a.name)),
                    a.cooldown,
                    a.energy,
                    ult,
                    max,
                    g,
                    d,
                    c,
                    e,
                    target,
                    radius
                ))
            })
            .collect();
        format!("[{}]", rows.join(","))
    }
}

/// Aim of catalog ability `ai` for the UI: its target word (`none`, `unit` or `point`, from the sim's
/// ability target codes 0, 1 and any other) and its cast range in whole world units, 0 for `none`
/// or an unknown ability. A missing index (`0xFFFF` on an item without an active) reads as `none`.
fn targeting(cat: &sim::Catalog, ai: usize) -> (&'static str, i32) {
    const TARGET_NONE: u8 = 0;
    const TARGET_UNIT: u8 = 1;
    let Some(a) = cat.abilities.get(ai) else {
        return ("none", 0);
    };
    let word = match a.target {
        TARGET_NONE => "none",
        TARGET_UNIT => "unit",
        _ => "point",
    };
    let range_milli = cat.ext.get(ai).map_or(0, |e| e.cast_range);
    let radius = if a.target == TARGET_NONE { 0 } else { range_milli.max(0) / 1000 };
    (word, radius)
}

fn replay_error(e: replay::ReplayError) -> JsValue {
    JsValue::from_str(&format!("replay: {e:?}"))
}

/// A JSON string literal for `s`.
/// The mods `load_mods` was given: a JSON array of {"name", "text"} objects, in order.
fn parse_mods(json: &str) -> Result<Vec<(String, String)>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("load_mods: bad JSON: {e}"))?;
    let list = v.as_array().ok_or("load_mods: expected a list of {name, text}")?;
    list.iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.get("name").and_then(|n| n.as_str()).ok_or_else(|| format!("load_mods[{i}]: name is missing"))?;
            let text = m.get("text").and_then(|t| t.as_str()).ok_or_else(|| format!("load_mods[{i}]: text is missing"))?;
            Ok((name.to_owned(), text.to_owned()))
        })
        .collect()
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
