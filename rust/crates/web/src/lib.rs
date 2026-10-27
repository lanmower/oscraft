use wasm_bindgen::prelude::*;

use replay::{Command, Recording, Replayer};

use mod_host::ModHost;

const LIVE_IMAGE: &str = "map";

const MAX_UNITS: usize = 1024;

#[wasm_bindgen]
pub struct Game {
    host: ModHost,
    image: Vec<u8>,
    replay: Option<Replayer>,
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

    pub fn replay_end(&self) -> u32 {
        self.replay.as_ref().map_or(0, Replayer::end)
    }

    pub fn replay_incomplete(&self) -> i32 {
        self.replay.as_ref().and_then(|p| p.recording().incomplete()).map_or(-1, |t| t as i32)
    }

    pub fn map_size(&self) -> Vec<u32> {
        let world = self.host.sim();
        let g = world.grid();
        vec![g.w as u32, g.h as u32]
    }

    pub fn tick(&mut self) -> String {
        if self.host.version() == 0 {
            self.rec.tick(&mut self.host.sim_mut(), true, true);
            return String::new();
        }
        self.host.advance().err().unwrap_or_default()
    }

    pub fn set_attr_rules(&mut self, text: &str) -> String {
        if self.host.sim_mut().set_attr_rules_text(text) {
            String::new()
        } else {
            "the pack's rules text is refused".to_owned()
        }
    }

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

    pub fn dispatch_action(&mut self, name: &str, args: &str) -> String {
        let args: serde_json::Value = match serde_json::from_str(if args.trim().is_empty() { "null" } else { args }) {
            Ok(v) => v,
            Err(e) => return format!("dispatch_action {name}: bad args JSON: {e}"),
        };
        self.host.dispatch_action(name, &args).err().unwrap_or_default()
    }

    pub fn pointer(&mut self, kind: &str, x: f64, y: f64, button: i32) -> String {
        if !x.is_finite() || !y.is_finite() {
            return format!("pointer {kind}: non-finite tile coordinate ({x}, {y})");
        }
        self.host.pointer(kind, x, y, i64::from(button)).err().unwrap_or_default()
    }

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

    pub fn save_state(&self) -> Vec<u8> {
        self.host.sim().save_state()
    }

    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.host.load_state(bytes).map_err(|e| JsValue::from_str(&e))?;
        self.rec.mark_incomplete(self.host.sim().ticks());
        Ok(())
    }

    pub fn unit_ids(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| i as u32)
            .collect()
    }

    pub fn unit_attacking(&self) -> Vec<u8> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| u8::from(self.host.sim().weapon_cooldown(i) > 0))
            .collect()
    }

    pub fn unit_strike_target(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().commanded_strike_index(i))
            .collect()
    }

    pub fn unit_periods(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().period_of(i))
            .collect()
    }

    pub fn unit_hps(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| sim::fixed::to_int(self.host.sim().hp(i)))
            .collect()
    }

    pub fn unit_max_hps(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| sim::fixed::to_int(self.host.sim().hp_max(i)))
            .collect()
    }

    pub fn unit_item_gen(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().item_generation(i))
            .collect()
    }

    pub fn ground_items(&self) -> Vec<f32> {
        self.host
            .sim()
            .ground_items()
            .iter()
            .flat_map(|g| {
                [g.id as f32, (g.kind - 1) as f32, g.count as f32, sim::fixed::to_float(g.x) as f32, sim::fixed::to_float(g.y) as f32]
            })
            .collect()
    }

    pub fn unit_scales(&self) -> Vec<u16> {
        (0..self.host.sim().count()).filter(|&i| self.host.sim().alive(i)).map(|i| self.host.sim().unit_scale(i)).collect()
    }

    pub fn unit_headings(&self) -> Vec<i32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| if self.host.sim().turn_step_of(i) > 0 { self.host.sim().heading_of(i) } else { -1 })
            .collect()
    }

    pub fn dead_props(&self) -> Vec<u32> {
        self.host.sim().dead_props()
    }

    pub fn order_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        const ORDER_MOVE: u8 = 0;
        let (Some(fx), Some(fy)) = (tile_fixed(x), tile_fixed(y)) else { return false };
        self.rec.apply(&mut self.host.sim_mut(), Command::Kind { unit: unit as usize, kind: ORDER_MOVE, x: fx, y: fy })
    }

    pub fn attack_move(&mut self, unit: u32, x: f32, y: f32) -> bool {
        let (Some(fx), Some(fy)) = (tile_fixed(x), tile_fixed(y)) else { return false };
        self.rec.apply(&mut self.host.sim_mut(), Command::Kind { unit: unit as usize, kind: sim::ORDER_ATTACK_MOVE, x: fx, y: fy })
    }

    pub fn hold(&mut self, unit: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::Hold { unit: unit as usize })
    }

    pub fn order_queue_push(&mut self, unit: u32, kind: u8, x: f32, y: f32, target: i32) -> bool {
        let (Some(fx), Some(fy)) = (tile_fixed(x), tile_fixed(y)) else { return false };
        self.host.sim_mut().queue_order(unit as usize, kind, fx, fy, target).is_ok()
    }

    pub fn order_queue_clear(&mut self, unit: u32) -> bool {
        self.host.sim_mut().queue_clear(unit as usize)
    }

    pub fn class_override(&self, unit: u32) -> i32 {
        self.host.sim().class_override_of(unit as usize).map_or(-1, i32::from)
    }

    pub fn hold_override(&self, unit: u32) -> i32 {
        self.host.sim().hold_override_of(unit as usize).map_or(-1, i32::from)
    }

    pub fn set_class_override(&mut self, unit: u32, class: i32) -> bool {
        let class = match class {
            -1 => 255,
            0..=254 => class as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.host.sim_mut(), Command::ClassOverride { unit, class })
    }

    pub fn set_hold_override(&mut self, unit: u32, hold: i32) -> bool {
        let hold = match hold {
            -1 => 255,
            0 | 1 => hold as u8,
            _ => return false,
        };
        self.rec.apply(&mut self.host.sim_mut(), Command::HoldOverride { unit, hold })
    }

    pub fn last_refusal(&self) -> String {
        format!("{:?}", self.host.sim().stats.last_refusal)
    }

    pub fn unit_hp(&self, unit: u32) -> i32 {
        sim::fixed::to_int(self.host.sim().hp(unit as usize))
    }

    pub fn xp(&self, unit: u32) -> u32 {
        self.host.sim().xp(unit as usize)
    }

    pub fn gold(&self, player: u8) -> i32 {
        self.host.sim().resource_a(player)
    }

    pub fn winner(&self) -> i32 {
        self.host.sim().winner().map_or(-1, |side| side as i32)
    }

    pub fn cast(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let (Some(fx), Some(fy)) = (tile_fixed(x), tile_fixed(y)) else { return false };
        if !self.host.cast_allowed(unit as usize, slot as usize, target, fx, fy, false) {
            return false;
        }
        self.rec.apply(&mut self.host.sim_mut(), Command::Cast { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    pub fn use_item(&mut self, unit: u32, slot: u32, target: i32, x: f32, y: f32) -> bool {
        let (Some(fx), Some(fy)) = (tile_fixed(x), tile_fixed(y)) else { return false };
        if !self.host.cast_allowed(unit as usize, slot as usize, target, fx, fy, true) {
            return false;
        }
        self.rec.apply(&mut self.host.sim_mut(), Command::UseItem { unit: unit as usize, slot: slot as usize, target, x: fx, y: fy })
    }

    pub fn skill_up(&mut self, unit: u32, slot: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::SkillUp { unit: unit as usize, slot: slot as usize })
    }

    pub fn hostile(&self, a: u32, b: u32) -> bool {
        self.host.sim().hostile(a as u8, b as u8)
    }

    pub fn attack(&mut self, unit: u32, target: u32) -> bool {
        self.rec.apply(&mut self.host.sim_mut(), Command::Attack { unit: unit as usize, target: target as usize })
    }

    pub fn hurt(&mut self, unit: u32, damage: i32) {
        self.host.sim_mut().hurt(unit as usize, sim::fixed::from_int(damage));
        self.rec.mark_incomplete(self.host.sim().ticks());
    }

    pub fn kills_of(&self, unit: u32) -> u32 {
        self.host.sim().kills_of(unit as usize)
    }

    pub fn deaths_of(&self, unit: u32) -> u32 {
        self.host.sim().deaths_of(unit as usize)
    }

    pub fn spawn_attributed(&mut self, player: u8, kind: u32, x: f32, y: f32) -> i32 {
        let fx = (x as f64 * 65536.0) as i32;
        let fy = (y as f64 * 65536.0) as i32;
        let id = self.host.sim_mut().spawn_attributed(player, kind, fx, fy).map_or(-1, |i| i as i32);
        self.rec.mark_incomplete(self.host.sim().ticks());
        id
    }

    pub fn remove_unit(&mut self, unit: u32) -> bool {
        let removed = self.host.sim_mut().remove_unit(unit as usize);
        self.rec.mark_incomplete(self.host.sim().ticks());
        removed
    }

    pub fn unit_gold(&self, unit: u32) -> i32 {
        self.host.sim().unit_gold(unit as usize)
    }

    pub fn ability_cooldown(&self, unit: u32, slot: u32) -> i32 {
        self.host.sim().ability_cooldown(unit as usize, slot as usize)
    }

    pub fn status_left(&self, unit: u32, kind: u32) -> i32 {
        self.host.sim().status_left(unit as usize, kind as usize)
    }

    pub fn stun_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 0)
    }

    pub fn slow_left(&self, unit: u32) -> i32 {
        self.host.sim().status_left(unit as usize, 1)
    }

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

    pub fn drain_kill_events(&mut self) -> Vec<u32> {
        self.host.sim_mut()
            .drain_kill_events()
            .into_iter()
            .flat_map(|e| [e.tick, e.killer, e.killer_player as u32, e.victim, e.victim_player as u32])
            .collect()
    }

    pub fn drain_ui(&mut self) -> String {
        self.host.flush_ui();
        let bodies = self.host.take_ui();
        format!("[{}]", bodies.iter().map(|b| format!("{{{b}}}")).collect::<Vec<_>>().join(","))
    }

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

    pub fn set_fog(&mut self, on: bool) {
        self.rec.apply(&mut self.host.sim_mut(), Command::Fog { on });
    }

    pub fn vision_dims(&self) -> Vec<u32> {
        let (w, h, s) = self.host.sim().vision_dims();
        vec![w as u32, h as u32, s as u32]
    }

    pub fn vision(&self, p: u8) -> Vec<u8> {
        self.host.sim().vision_plane(p)
    }

    pub fn vision_generation(&self) -> u32 {
        self.host.sim().vision_generation()
    }

    pub fn fog_on(&self) -> bool {
        self.host.sim().fog_on()
    }

    pub fn day_cycle(&self) -> Vec<u32> {
        self.host.sim().day_cycle().to_vec()
    }

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

    pub fn unit_types(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().unit_type(i).max(0) as u32)
            .collect()
    }

    pub fn unit_filter_bits(&self) -> Vec<u32> {
        (0..self.host.sim().count())
            .filter(|&i| self.host.sim().alive(i))
            .map(|i| self.host.sim().filter_bits(i))
            .collect()
    }

    pub fn unit_model_names(&self) -> Vec<String> {
        let world = self.host.sim();
        let cat = world.catalog();
        cat.units.iter().map(|u| cat.name(u.model).to_owned()).collect()
    }

    pub fn unit_type_names(&self) -> Vec<String> {
        let world = self.host.sim();
        let cat = world.catalog();
        cat.units.iter().map(|u| cat.name(u.name).to_owned()).collect()
    }

    pub fn unit_flags(&self) -> Vec<u8> {
        self.host.sim().catalog().unit_ext.iter().map(|e| e.player_controlled as u8).collect()
    }

    pub fn give_item(&mut self, unit: u32, item: u32, count: u32) -> bool {
        let Some(kind) = item.checked_add(1) else { return false };
        let added = self.host.sim_mut().inventory_add(unit as usize, kind, count);
        self.rec.mark_incomplete(self.host.sim().ticks());
        added
    }

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
                        let n = (s.max_rank as usize).min(s.gates.len());
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

fn tile_fixed(tile: f32) -> Option<i32> {
    let scaled = f64::from(tile) * 65536.0;
    (scaled.is_finite() && scaled >= f64::from(i32::MIN) && scaled <= f64::from(i32::MAX)).then(|| scaled as i32)
}

fn targeting(cat: &sim::Catalog, ai: usize) -> (&'static str, i32) {
    const TARGET_NONE: u8 = 0;
    const TARGET_UNIT: u8 = 1;
    const TARGET_UNIT_OR_POINT: u8 = 3;
    let Some(a) = cat.abilities.get(ai) else {
        return ("none", 0);
    };
    let word = match a.target {
        TARGET_NONE => "none",
        TARGET_UNIT => "unit",
        TARGET_UNIT_OR_POINT => "unit_or_point",
        _ => "point",
    };
    let range_milli = cat.ext.get(ai).map_or(0, |e| e.cast_range);
    let radius = if a.target == TARGET_NONE { 0 } else { range_milli.max(0) / 1000 };
    (word, radius)
}

fn replay_error(e: replay::ReplayError) -> JsValue {
    JsValue::from_str(&format!("replay: {e:?}"))
}

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
