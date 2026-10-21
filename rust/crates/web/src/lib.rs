//! Browser entry point. `Game` loads a map image (the bytes sim::load reads) and steps the sim;
//! the page draws the units it reports.

use wasm_bindgen::prelude::*;

// The replay format and player are shared with the server crate, which records the files this plays.
#[allow(dead_code)]
#[path = "../../server/src/replay.rs"]
mod replay;
use replay::{Command, Recording, Replayer};

/// The image name the live recording runs on. The browser loads map bytes, so it has no file name to record.
const LIVE_IMAGE: &str = "map";

/// The unit cap the gate uses, so the browser and the parity checks load the same limits.
const MAX_UNITS: usize = 1024;

#[wasm_bindgen]
pub struct Game {
    sim: sim::Sim,
    image: Vec<u8>,
    replay: Option<Replayer>,
    /// The commands the live sim received, so a data swap re-bases the record the same way the server's does.
    rec: Recording,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(image: &[u8]) -> Result<Game, JsValue> {
        let sim = sim::load(image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        let rec = Recording::new(LIVE_IMAGE, MAX_UNITS, &sim);
        Ok(Game { sim, image: image.to_vec(), replay: None, rec })
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
        self.sim = fresh;
        self.replay = Some(player);
        Ok(end)
    }

    /// Advances the loaded replay by up to `n` ticks, checking each recorded hash on the way.
    /// Stops at the end of the replay. Returns the tick reached.
    pub fn replay_step(&mut self, n: u32) -> Result<u32, JsValue> {
        let player = self.replay.as_mut().ok_or_else(|| JsValue::from_str("no replay loaded"))?;
        for _ in 0..n {
            if self.sim.ticks() >= player.end() {
                break;
            }
            player.step(&mut self.sim).map_err(replay_error)?;
        }
        Ok(self.sim.ticks())
    }

    /// Moves the loaded replay to `tick`. Going back re-simulates from tick 0 on a fresh load.
    pub fn replay_seek(&mut self, tick: u32) -> Result<u32, JsValue> {
        let player = self.replay.as_mut().ok_or_else(|| JsValue::from_str("no replay loaded"))?;
        let tick = tick.min(player.end());
        if tick < self.sim.ticks() {
            let fresh = sim::load(&self.image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
            player.reset(&fresh).map_err(replay_error)?;
            self.rec.mark_incomplete(fresh.ticks());
            self.sim = fresh;
        }
        player.seek(&mut self.sim, tick).map_err(replay_error)
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
        let g = self.sim.grid();
        vec![g.w as u32, g.h as u32]
    }

    pub fn tick(&mut self) {
        self.rec.tick(&mut self.sim, true, true);
    }

    /// Swaps the live match onto the map image `image` (new data). The live units carry over onto it, as on the server,
    /// and the record starts a new segment at this tick. Returns `[carried, refused]`: the live units carried, and the
    /// live units the new data had no slot for. Refused when the image does not load; the match is unchanged then.
    pub fn swap_data(&mut self, image: &[u8]) -> Result<Vec<u32>, JsValue> {
        let mut fresh = sim::load(image, MAX_UNITS).map_err(|e| JsValue::from_str(&format!("sim::load: {e:?}")))?;
        fresh.set_gold_by_mod(self.sim.gold_by_mod());
        fresh.set_fog(self.sim.fog_on());
        let (carried, refused) = sim::carry_from(&self.sim, &mut fresh);
        self.rec.rebase(LIVE_IMAGE, MAX_UNITS, &fresh);
        self.sim = fresh;
        self.image = image.to_vec();
        Ok(vec![carried as u32, refused as u32])
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
        self.sim.load_state(bytes).map_err(|e| JsValue::from_str(&format!("load_state: {e:?}")))?;
        self.rec.mark_incomplete(self.sim.ticks());
        Ok(())
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
        self.rec.apply(&mut self.sim, Command::Kind { unit: unit as usize, kind: ORDER_MOVE, x: fx, y: fy })
    }

    /// Attack-moves unit `unit` to tile (x, y): it walks there, strikes each enemy in its reach on the way,
    /// and resumes travel after each kill. Returns false when the sim refuses the order.
    pub fn attack_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.rec.apply(&mut self.sim, Command::Kind { unit: unit as usize, kind: sim::ORDER_ATTACK_MOVE, x: fx, y: fy })
    }

    /// Holds unit `unit` where it stands: it does not chase, and strikes only enemies in its reach.
    /// Returns false when the sim refuses the order (dead or unknown unit, stunned).
    pub fn hold(&mut self, unit: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::Hold { unit: unit as usize })
    }

    /// Debug view: the target class a mod set on unit `unit` (0..=3), or -1 when the Rust rule applies.
    pub fn class_override(&self, unit: u32) -> i32 {
        self.sim.class_override_of(unit as usize).map_or(-1, i32::from)
    }

    /// Debug view: the attack-move hold a mod set on unit `unit` (1 always holds, 0 never), or -1 when the Rust rule applies.
    pub fn hold_override(&self, unit: u32) -> i32 {
        self.sim.hold_override_of(unit as usize).map_or(-1, i32::from)
    }

    /// Sets unit `unit`'s target class to `class` (0..=3), or clears it with -1. The change goes through the replay
    /// command the mods use, so a recording and the peers that replay it see the same change. False when the sim refuses it.
    pub fn set_class_override(&mut self, unit: u32, class: i32) -> bool {
        let class = match class {
            -1 => 255,
            0..=254 => class as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.sim, Command::ClassOverride { unit, class })
    }

    /// Sets unit `unit`'s attack-move hold: 1 always holds, 0 never holds, -1 restores the Rust rule.
    /// Same replay command path as `set_class_override`. False when the sim refuses it.
    pub fn set_hold_override(&mut self, unit: u32, hold: i32) -> bool {
        let hold = match hold {
            -1 => 255,
            0 | 1 => hold as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.sim, Command::HoldOverride { unit, hold })
    }

    /// Name of the sim's most recent refusal (`Refusal` variant), for the page's refusal witness.
    pub fn last_refusal(&self) -> String {
        format!("{:?}", self.sim.stats.last_refusal)
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
        self.rec.apply(&mut self.sim, Command::Cast { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    /// Uses the active item in inventory slot `slot` of unit `unit`, aimed at unit `target` or at the
    /// ground (tile x, y) when `target` is -1. Returns false when the sim refuses it (empty slot, item
    /// cooldown, range); a refused use changes nothing.
    pub fn use_item(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        self.rec.apply(&mut self.sim, Command::UseItem { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    /// Spends one skill point to raise ability slot `slot` of unit `unit` by a rank. Returns false
    /// when the sim refuses it (no free point, max rank, level gate, or no skill tree).
    pub fn skill_up(&mut self, unit: u32, slot: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::SkillUp { unit: unit as usize, slot: slot as usize })
    }

    /// Chooses talent `side` (0 or 1) of tier `tier` (0..=3, levels 10, 15, 20, 25) for unit `unit`. Returns false
    /// when the sim refuses it (level below the tier, tier already chosen, dead unit).
    pub fn choose_talent(&mut self, unit: u32, tier: u32, side: u32) -> bool {
        let chosen = self.sim.choose_talent(unit as usize, tier as usize, side as usize).is_ok();
        self.rec.mark_incomplete(self.sim.ticks());
        chosen
    }

    /// Upgrade bits of unit `unit`: bit 0 scepter, bit 1 shard, bits 2..=9 the eight talent choices (tier * 2 + side).
    pub fn upgrade_bits(&self, unit: u32) -> i32 {
        self.sim.upgrade_bits(unit as usize)
    }

    /// Rank of ability slot `slot` of unit `unit`; 0 when not learned.
    pub fn skill_rank(&self, unit: u32, slot: u32) -> u32 {
        self.sim.skill_rank(unit as usize, slot as usize) as u32
    }

    /// Unspent skill points of unit `unit`.
    pub fn skill_points(&self, unit: u32) -> u32 {
        self.sim.skill_points(unit as usize)
    }

    /// True when player `a` is an enemy of player `b`: different teams and no alliance.
    pub fn hostile(&self, a: u32, b: u32) -> bool {
        self.sim.hostile(a as u8, b as u8)
    }

    /// Orders unit `unit` to attack unit `target`, walking toward it when it is out of reach.
    /// Returns false when the sim refuses the order.
    pub fn attack(&mut self, unit: u32, target: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::Attack { unit: unit as usize, target: target as usize })
    }

    /// Test hook for the page's CI probe: deals `damage` whole points to unit `unit`.
    pub fn hurt(&mut self, unit: u32, damage: i32) {
        self.sim.hurt(unit as usize, sim::fixed::from_int(damage));
        self.rec.mark_incomplete(self.sim.ticks());
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
        let id = self.sim.spawn_hero(player, kind, fx, fy).map_or(-1, |i| i as i32);
        self.rec.mark_incomplete(self.sim.ticks());
        id
    }

    /// Removes live unit `unit` from the sim: its slot stops counting and no query finds it. False when it is not alive.
    pub fn remove_unit(&mut self, unit: u32) -> bool {
        let removed = self.sim.remove_unit(unit as usize);
        self.rec.mark_incomplete(self.sim.ticks());
        removed
    }

    /// Loads the hero rules from `key=value` text (scenarios/moba/hero_rules.txt). False, rules unchanged, on a bad line.
    pub fn set_hero_rules(&mut self, text: &str) -> bool {
        let set = self.sim.set_hero_rules_text(text);
        self.rec.mark_incomplete(self.sim.ticks());
        set
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
        let bought = self.sim.buyback(unit as usize);
        self.rec.mark_incomplete(self.sim.ticks());
        bought
    }

    /// Gold held by hero `unit`; 0 for a non-hero.
    pub fn hero_gold(&self, unit: u32) -> i32 {
        self.sim.hero_gold(unit as usize)
    }

    /// Experience held by unit `unit`; 0 for an unknown unit.
    pub fn xp(&self, unit: u32) -> u32 {
        self.sim.xp(unit as usize)
    }

    /// Level of unit `unit`: the number of xp curve thresholds its xp has reached.
    pub fn level(&self, unit: u32) -> u32 {
        self.sim.level(unit as usize)
    }

    /// Strength, agility and intelligence of hero `unit` as `[str, agi, int]`; zeros for a non-hero.
    pub fn attributes(&self, unit: u32) -> Vec<i32> {
        self.sim.hero_attributes(unit as usize).to_vec()
    }

    /// Max mana of hero `unit` from its intelligence; 0 for a non-hero. A stat only: no mana pool yet.
    pub fn max_mana(&self, unit: u32) -> i32 {
        self.sim.hero_max_mana(unit as usize)
    }

    /// Ticks left on ability `slot` of unit `unit` (0 when ready); -1 for an unknown unit or slot.
    pub fn ability_cooldown(&self, unit: u32, slot: u32) -> i32 {
        self.sim.ability_cooldown(unit as usize, slot as usize)
    }

    /// Ticks left on a status of `unit`: `kind` 0 stun, 1 slow, 2 silence, 3 root, 4 invisibility
    /// (`sim::STATUS_NAMES`). 0 when the status is off or the unit or kind is unknown.
    pub fn status_left(&self, unit: u32, kind: u32) -> i32 {
        self.sim.status_left(unit as usize, kind as usize)
    }

    pub fn stun_left(&self, unit: u32) -> i32 {
        self.sim.status_left(unit as usize, 0)
    }

    pub fn slow_left(&self, unit: u32) -> i32 {
        self.sim.status_left(unit as usize, 1)
    }

    /// The slow's speed multiplier, Q16 (65536 when the unit is not slowed).
    pub fn slow_multiplier(&self, unit: u32) -> i32 {
        self.sim.slow_multiplier(unit as usize)
    }

    pub fn silence_left(&self, unit: u32) -> i32 {
        self.sim.status_left(unit as usize, 2)
    }

    pub fn root_left(&self, unit: u32) -> i32 {
        self.sim.status_left(unit as usize, 3)
    }

    pub fn invisible_left(&self, unit: u32) -> i32 {
        self.sim.status_left(unit as usize, 4)
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
        self.rec.apply(&mut self.sim, Command::Fog { on });
    }

    // fogview: vision grid [width, height, cell size in tiles] and player `p`'s visible cells, one byte each.
    pub fn vision_dims(&self) -> Vec<u32> {
        let (w, h, s) = self.sim.vision_dims();
        vec![w as u32, h as u32, s as u32]
    }

    pub fn vision(&self, p: u8) -> Vec<u8> {
        self.sim.vision_plane(p)
    }

    // fogview: changes whenever a visible plane may have changed; equal values mean `vision` would repeat its bytes.
    pub fn vision_generation(&self) -> u32 {
        self.sim.vision_generation()
    }

    pub fn fog_on(&self) -> bool {
        self.sim.fog_on()
    }

    // fogview: day and night clock as `[ticks into cycle, day ticks, night ticks]`.
    pub fn day_cycle(&self) -> Vec<u32> {
        vec![self.sim.day_clock(), sim::DAY_TICKS, sim::NIGHT_TICKS]
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

    /// The catalog name of each unit type, indexed by type index: the index `spawn_hero` takes.
    pub fn unit_type_names(&self) -> Vec<String> {
        let cat = self.sim.catalog();
        cat.units.iter().map(|u| cat.name(u.name).to_owned()).collect()
    }

    /// Per unit type, indexed like unit_type_names: 1 when the type is a hero (player-controlled: unit_extra
    /// player_controlled=True, the flag that enrolls a placed unit as a hero), 0 for every other kind.
    pub fn unit_hero_kinds(&self) -> Vec<u8> {
        self.sim.catalog().unit_ext.iter().map(|e| e.player_controlled as u8).collect()
    }

    /// UI export: buys catalog item index `item` for unit `unit` at a shop in reach. False when refused.
    pub fn buy(&mut self, unit: u32, item: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::Buy { unit: unit as usize, item: item as usize })
    }

    /// Test hook for the page's CI probe: puts `count` charges of catalog item index `item` into unit `unit`'s inventory. False when no slot is free.
    pub fn give_item(&mut self, unit: u32, item: u32, count: u32) -> bool {
        let added = self.sim.inventory_add(unit as usize, item + 1, count);
        self.rec.mark_incomplete(self.sim.ticks());
        added
    }

    /// UI export: sells inventory slot `slot` of unit `unit` for half its cost. False when refused.
    pub fn sell(&mut self, unit: u32, slot: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::Sell { unit: unit as usize, slot: slot as usize })
    }

    /// UI export: moves stash slot `slot` of unit `unit` into the inventory. False when refused.
    pub fn unstash(&mut self, unit: u32, slot: u32) -> bool {
        self.rec.apply(&mut self.sim, Command::Unstash { unit: unit as usize, slot: slot as usize })
    }

    /// UI export: character level of unit `unit` (xp level plus one), the number skill gates compare against.
    pub fn character_level(&self, unit: u32) -> u32 {
        self.sim.character_level(unit as usize)
    }

    /// UI export: the item catalog as JSON `[{"id":index,"name","cost","armor","damage","health","flags",
    /// "tier","shops","charges","stack","stats":{name:value,...},"components":[index,...],"target","radius"}]`
    /// (`target` and `radius` describe the item's active ability, see `targeting`). `stats` lists
    /// the non-zero bonuses; `shops` is the bit mask home 1, side 2, secret 4.
    pub fn item_catalog_json(&self) -> String {
        let cat = self.sim.catalog();
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
                    "{{\"id\":{},\"name\":{},\"cost\":{},\"armor\":{},\"damage\":{},\"health\":{},\"flags\":{},\"tier\":{},\"shops\":{},\"charges\":{},\"stack\":{},\"stats\":{{{}}},\"components\":[{}],\"target\":\"{}\",\"radius\":{}}}",
                    id,
                    json_str(cat.name(it.name)),
                    it.cost,
                    it.stats[0],
                    it.stats[1],
                    it.stats[2],
                    it.flags,
                    it.tier,
                    it.shops,
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
            let cells: Vec<String> = (0..self.sim.inventory_slots())
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
            bank(&|s| self.sim.inventory_slot(u, s)),
            bank(&|s| self.sim.stash_slot(u, s))
        )
    }

    /// UI export: the abilities unit `unit` holds as JSON `[{"slot","name","cooldown","energy","ultimate","max_rank",
    /// "gates":[...],"damage":[...],"cooldowns":[...],"energies":[...],"target":"none|unit|point","radius"}]`
    /// (world units, 0 for `none`); ability stats are game ticks and
    /// the skill arrays are empty for an ability with no skill tree.
    pub fn abilities_json(&self, unit: u32) -> String {
        let cat = self.sim.catalog();
        let list = |v: &[i32]| v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",");
        let rows: Vec<String> = (0..sim::ABILITY_SLOTS)
            .filter_map(|slot| {
                let ai = self.sim.ability_in_slot(unit as usize, slot)?;
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
