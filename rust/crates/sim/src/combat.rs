use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::*;

pub(crate) const KILL_QUEUE_CAP: usize = 256;
pub(crate) const HITTER_SLOTS: usize = 4;
pub const COUNTER_DAMAGE_DEALT: &str = "sim_damage_dealt";
pub const COUNTER_DAMAGE_TAKEN: &str = "sim_damage_taken";
pub const COUNTER_HEALING: &str = "sim_healing";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KillEvent {
    pub tick: u32,
    pub killer: u32,
    pub killer_player: u8,
    pub victim: u32,
    pub victim_player: u8,
}
impl Sim {
    pub(crate) fn armour_scaled(&self, damage: i32, target: usize) -> i32 {
        ((damage as i64 * self.armour_scale[target] as i64) >> fixed::SHIFT).clamp(0, i32::MAX as i64) as i32
    }

    pub fn set_armour_scale(&mut self, i: usize, scale: i32) {
        if i < self.n {
            self.armour_scale[i] = scale;
        }
    }

    pub fn armour_scale_of(&self, i: usize) -> i32 {
        self.armour_scale.get(i).copied().unwrap_or(fixed::ONE)
    }

    pub fn eff_armour_of(&self, i: usize) -> i32 {
        if i < self.n { self.eff_armor(i) } else { 0 }
    }

    pub fn set_miss_chance(&mut self, i: usize, chance: i32) {
        if i < self.n {
            self.miss_chance[i] = chance;
        }
    }

    pub fn miss_chance_of(&self, i: usize) -> i32 {
        self.miss_chance.get(i).copied().unwrap_or(0)
    }

    pub fn data_miss_chance(&self, i: usize) -> i32 {
        if i >= self.n {
            return 0;
        }
        self.cat.unit_ext.get(self.type_of[i] as usize).map_or(0, |e| e.miss_chance)
    }

    pub(crate) fn final_factor(&self, attacker: usize, target: usize) -> i32 {
        let dtype = self.cat.unit_ext.get(self.type_of[attacker] as usize).map_or(0, |e| e.damage_type);
        let size = self.cat.unit_ext.get(self.type_of[target] as usize).map_or(0, |e| e.size);
        self.cat
            .damage_factors
            .iter()
            .find(|f| f.damage_type == dtype && f.size == size)
            .map_or(fixed::ONE, |f| f.factor)
    }

    pub fn scale_hit(&self, dmg: i32, attacker: usize, target: usize) -> i32 {
        ((dmg as i64 * self.final_factor(attacker, target) as i64) >> fixed::SHIFT) as i32
    }

    pub fn misses(&mut self, i: usize) -> bool {
        let blind = if self.status_left(i, crate::status::BLIND) > 0 { self.spell.blind_pct[i] * 65536 / 100 } else { 0 };
        let chance = self.miss_chance[i].max(blind);
        if chance <= 0 {
            return false;
        }
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let roll = ((self.rng >> 33) & 0xFFFF) as i32;
        roll < chance
    }

    fn hidden(&self, j: usize) -> bool {
        self.cat.unit_ext.get(self.type_of[j] as usize).map_or(false, |e| e.cloak & 1 != 0)
    }

    pub fn detected(&self, p: u8, j: usize) -> bool {
        self.status_left(j, crate::status::REVEALED) > 0 || (0..self.n).any(|k| {
            if self.alive[k] == 0 || !self.shares_sight(p, self.player[k]) {
                return false;
            }
            let radius = self.cat.unit_ext.get(self.type_of[k] as usize).map_or(0, |e| e.detector_radius).max(self.item_stat(k, crate::inventory::STAT_TRUE_SIGHT));
            if radius <= 0 {
                return false;
            }
            let reach = fixed::from_milli(radius) as i64;
            dist2(self.pos_x[k], self.pos_y[k], self.pos_x[j], self.pos_y[j]) <= reach * reach
        })
    }

    pub fn targetable(&self, p: u8, j: usize) -> bool {
        !self.hidden(j) || self.detected(p, j)
    }

    pub(crate) fn attack(&mut self, i: usize) {
        if self.damage[i] <= 0 || self.stunned(i) || self.disarmed(i) {
            return;
        }
        if self.cooldown[i] > 0 {
            self.cooldown[i] -= 1;
            if self.cooldown[i] > 0 {
                return;
            }
        }
        let Some(b) = self.commanded_strike(i) else {
            return;
        };
        if self.walking(i) {
            return;
        }
        if !self.face_target(i, b) {
            return;
        }
        self.cooldown[i] = self.attack_period(i);
        self.launch_strike(i, b, false);
    }

    pub(crate) fn engaged(&self, i: usize) -> bool {
        self.hold_override[i] == 1 && self.order_kind[i] == ORDER_ATTACK_MOVE
    }

    pub fn strike_lands(&self, j: usize) -> bool {
        self.status_left(j, crate::status::PHYSICAL_IMMUNE) == 0 && !self.invulnerable(j) && self.response_keep_of(j) != 0
    }

    pub fn can_target(&self, i: usize, j: usize) -> bool {
        i != j
            && i < self.n
            && j < self.n
            && self.alive[j] != 0
            && self.hostile(self.player[i], self.player[j])
            && (self.targets[i] & self.layer_of(j)) != 0
            && self.visible(self.player[i], j)
            && self.targetable(self.player[i], j)
    }

    pub fn denies(&self, i: usize, j: usize) -> bool {
        if i == j || i >= self.n || j >= self.n || self.alive[j] == 0 || self.hostile(self.player[i], self.player[j]) {
            return false;
        }
        let pct = self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| i64::from(e.deny_pct));
        (self.targets[i] & self.layer_of(j)) != 0 && (self.hp[j] as i64) * 100 < pct * (self.hp_max[j] as i64)
    }

    pub(crate) fn strike_ready(&self, i: usize, j: usize) -> bool {
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        self.damage[i] > 0 && (self.can_target(i, j) || self.denies(i, j)) && self.strike_measure(i, j).1 <= reach
    }

    pub fn can_strike(&self, i: usize, j: usize) -> bool {
        !self.stunned(i) && !self.disarmed(i) && self.strike_ready(i, j)
    }

    pub fn weapon(&self, i: usize) -> (i32, i32) {
        (self.damage[i], self.range_q[i])
    }

    pub fn set_hold_fire(&mut self, i: usize, on: bool) -> bool {
        if i >= self.n {
            return false;
        }
        self.hold_fire[i] = u8::from(on);
        true
    }

    pub fn holds_fire(&self, i: usize) -> bool {
        self.hold_fire.get(i).is_some_and(|&v| v != 0)
    }

    pub fn strike_of(&self, i: usize) -> i32 {
        self.strike_target[i]
    }

    pub(crate) fn commanded_strike(&self, i: usize) -> Option<usize> {
        if self.damage[i] <= 0 || self.stunned(i) || self.disarmed(i) || self.hold_fire[i] != 0 {
            return None;
        }
        let t = self.strike_target[i];
        if t < 0 || t as usize >= self.n {
            return None;
        }
        let j = t as usize;
        self.strike_ready(i, j).then_some(j)
    }

    pub fn commanded_strike_index(&self, i: usize) -> i32 {
        self.commanded_strike(i).map_or(-1, |j| j as i32)
    }

    pub fn set_strike_target(&mut self, i: usize, target: i32) -> Result<(), &'static str> {
        if i >= self.n {
            return Err("invalid_unit");
        }
        if target >= self.n as i32 {
            return Err("invalid_target");
        }
        self.strike_target[i] = target.max(-1);
        Ok(())
    }

    pub fn order_attack(&mut self, i: usize, target: usize) -> Result<(), crate::rules::Refusal> {
        use crate::rules::Refusal;
        if i >= self.n || self.alive[i] == 0 || target >= self.n || self.alive[target] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if self.stunned(i) {
            return Err(self.reject(Refusal::Stunned));
        }
        if self.disarmed(i) {
            return Err(self.reject(Refusal::Stunned));
        }
        if !self.strike_ready(i, target) {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.cooldown[i] > 1 {
            return Err(self.reject(Refusal::Cooldown));
        }
        if self.hold_fire[i] != 0 || !self.face_target(i, target) {
            return Ok(());
        }
        self.cooldown[i] = self.attack_period(i);
        self.launch_strike(i, target, true);
        Ok(())
    }

    pub fn hurt(&mut self, j: usize, dmg: i32) {
        self.hurt_by(j, dmg, -1);
    }

    fn magic_reduced(&self, j: usize, dmg: i32) -> i32 {
        self.magic_scaled(dmg, j)
    }

    pub(crate) fn magic_scaled(&self, dmg: i32, target: usize) -> i32 {
        (dmg as i64 * self.magic_keep[target] as i64 / 100) as i32
    }

    pub(crate) fn bonus_magic_scaled(&self, dmg: i32, target: usize) -> i32 {
        let keep = (100 - self.stat(target, crate::spell::S_MAGIC_RESIST)).clamp(0, 200);
        (dmg as i64 * keep as i64 / 100) as i32
    }

    pub(crate) fn attack_factor_scaled(&self, dmg: i32, attack: u8, target: usize) -> i32 {
        let size = self.cat.unit_ext.get(self.type_of[target] as usize).map_or(0, |e| e.size);
        let factor = self
            .cat
            .damage_factors
            .iter()
            .find(|f| f.damage_type == attack && f.size == size)
            .map_or(fixed::ONE, |f| f.factor);
        ((dmg as i64 * factor as i64) >> fixed::SHIFT) as i32
    }

    pub fn set_magic_keep(&mut self, i: usize, pct: i32) {
        if i < self.n {
            self.magic_keep[i] = pct;
        }
    }

    pub fn magic_keep_of(&self, i: usize) -> i32 {
        self.magic_keep.get(i).copied().unwrap_or(100)
    }

    pub fn magic_base_of(&self, i: usize) -> i32 {
        self.attr_def(i).map_or(0, |d| d.magic_resist)
    }

    pub fn set_damage_keep(&mut self, i: usize, pct: i32) {
        if i < self.n {
            self.damage_keep[i] = pct;
        }
    }

    pub fn damage_keep_of(&self, i: usize) -> i32 {
        self.damage_keep.get(i).copied().unwrap_or(100)
    }

    pub fn set_response_keep(&mut self, i: usize, pct: i32) {
        if i < self.n {
            self.response_keep[i] = pct;
        }
    }

    pub fn response_keep_of(&self, i: usize) -> i32 {
        self.response_keep.get(i).copied().unwrap_or(100)
    }

    pub fn response_of(&self, i: usize) -> i32 {
        if i < self.n { i32::from(self.response[i]) } else { 0 }
    }

    pub(crate) fn hurt_by(&mut self, j: usize, dmg: i32, attacker: i32) {
        if j >= self.n {
            return;
        }
        let dmg = self.magic_reduced(j, dmg);
        self.hurt_typed(j, dmg, attacker);
    }

    pub(crate) fn hurt_typed(&mut self, j: usize, dmg: i32, attacker: i32) {
        if j >= self.n {
            return;
        }
        let before = self.hp[j];
        self.inflict(j, dmg, attacker);
        self.stats.ability_damage = self.stats.ability_damage.wrapping_add(before - self.hp[j]);
    }

    pub fn roll_bounty(&mut self, j: usize) -> i32 {
        if j >= self.n {
            return 0;
        }
        let Some(ext) = self.cat.unit_ext.get(self.type_of[j] as usize).copied() else { return 0 };
        if ext.bounty_max <= ext.bounty_min {
            return ext.bounty;
        }
        ext.bounty_min + self.rand_below((ext.bounty_max - ext.bounty_min + 1) as u32) as i32
    }

    pub(crate) fn inflict(&mut self, j: usize, mut dmg: i32, attacker: i32) {
        self.touch(j);
        if self.invulnerable(j) {
            dmg = 0;
        } else if self.is_illusion(j) {
            dmg = (dmg as i64 * self.spell.ill_in[j] as i64 / 100) as i32;
        }
        if dmg > 0 {
            dmg = (dmg as i64 * self.damage_keep[j] as i64 / 100) as i32;
            self.trig_hit[j] = 1;
            if attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j {
                self.remember_hitter(j, attacker);
            }
            if self.spell.damage_interrupts {
                self.interrupt_channel(j);
            }
            self.last_hit[j] = self.ticks;
            dmg = (dmg as i64 * self.response_keep[j] as i64 / 100) as i32;
            dmg = self.mitigate_mech(j, dmg);
            dmg = self.absorb_shield(j, dmg);
            if attacker >= 0 && attacker as usize != j && dmg > self.spell.heavy_hit_min {
                self.heavy_hit[j] = self.ticks.wrapping_add(1);
            }
        }
        if dmg >= self.hp[j] && self.status_left(j, crate::status::DEATH_PREVENTION) > 0 {
            dmg = (self.hp[j] - 1).max(0);
        }
        let dealt = dmg.min(self.hp[j]).max(0);
        self.hp[j] = self.hp[j].wrapping_sub(dmg);
        self.log_hit(attacker, j, dealt);
        self.tally_attributed(j, COUNTER_DAMAGE_TAKEN, dealt);
        if attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j {
            self.tally_attributed(attacker as usize, COUNTER_DAMAGE_DEALT, dealt);
        }
        if self.hp[j] <= 0 {
            let fell = self.alive[j] != 0;
            self.hp[j] = 0;
            self.alive[j] = 0;
            self.trig_death[j] = 1;
            self.stats.deaths += 1;
            if fell && !self.is_illusion(j) {
                self.record_death(j, attacker);
                self.hold_dead(j);
                self.group_died(j);
            }
        }
    }

    pub(crate) fn record_death(&mut self, j: usize, attacker: i32) {
        self.unit_deaths[j] = self.unit_deaths[j].saturating_add(1);
        for u in 0..self.n {
            if self.attack_order[u] == j as i32 {
                self.attack_order[u] = -1;
            }
        }
        if attacker < 0 || attacker as usize >= self.n || attacker as usize == j {
            self.killer[j] = -1;
            self.hitter_id[j * HITTER_SLOTS..(j + 1) * HITTER_SLOTS].fill(-1);
            return;
        }
        let k = attacker as usize;
        self.killer[j] = attacker;
        if self.kill_queue.len() < KILL_QUEUE_CAP {
            self.kill_queue.push(KillEvent {
                tick: self.ticks,
                killer: k as u32,
                killer_player: self.player[k],
                victim: j as u32,
                victim_player: self.player[j],
            });
        }
    }

    fn remember_hitter(&mut self, j: usize, hitter: i32) {
        let base = j * HITTER_SLOTS;
        let slots = base..base + HITTER_SLOTS;
        let slot = slots
            .clone()
            .find(|&s| self.hitter_id[s] == hitter)
            .or_else(|| slots.clone().find(|&s| self.hitter_id[s] < 0))
            .unwrap_or_else(|| slots.min_by_key(|&s| self.hitter_tick[s]).unwrap_or(base));
        self.hitter_id[slot] = hitter;
        self.hitter_tick[slot] = self.ticks;
    }

    pub fn recent_hitters(&self, j: usize, window: u32) -> Vec<u32> {
        if j >= self.n {
            return Vec::new();
        }
        let base = j * HITTER_SLOTS;
        (base..base + HITTER_SLOTS)
            .filter_map(|s| {
                let id = self.hitter_id[s];
                (id >= 0 && (id as usize) < self.n && self.ticks.wrapping_sub(self.hitter_tick[s]) <= window).then_some(id as u32)
            })
            .collect()
    }

    pub fn take_recent_hitters(&mut self, j: usize, window: u32) -> Vec<u32> {
        let out = self.recent_hitters(j, window);
        if j < self.n {
            let base = j * HITTER_SLOTS;
            self.hitter_id[base..base + HITTER_SLOTS].fill(-1);
        }
        out
    }

    pub fn heavy_hit_age(&self, j: usize) -> Option<u32> {
        let stamp = *self.heavy_hit.get(j)?;
        (stamp != 0).then(|| self.ticks.wrapping_sub(stamp.wrapping_sub(1)))
    }

    pub fn hit_by(&mut self, j: usize, dmg: i32, attacker: i32) {
        self.inflict(j, dmg, attacker);
    }

    pub(crate) fn tally_attributed(&mut self, i: usize, name: &str, amount: i32) {
        if amount > 0 && self.attr_def(i).is_some() {
            let total = self.counter(i, name).saturating_add(i64::from(amount));
            self.set_counter(i, name, total);
        }
    }

    pub fn add_kill(&mut self, i: usize, n: u32) {
        if i < self.n {
            self.unit_kills[i] = self.unit_kills[i].saturating_add(n);
        }
    }

    pub fn add_credit(&mut self, i: usize, n: u32) {
        if i < self.n {
            self.unit_credits[i] = self.unit_credits[i].saturating_add(n);
        }
    }

    pub fn credits_of(&self, i: usize) -> u32 {
        self.unit_credits.get(i).copied().unwrap_or(0)
    }

    pub fn drain_kill_events(&mut self) -> Vec<KillEvent> {
        self.kill_queue.drain(..).collect()
    }

    pub fn damage_target(&mut self, caster: i32, p: u8, dmg: i32, target: i32) {
        if dmg <= 0 || target < 0 || target as usize >= self.n {
            return;
        }
        let j = target as usize;
        if target == caster || self.alive[j] == 0 || !self.hostile(p, self.player[j]) {
            return;
        }
        self.hurt_by(j, dmg, caster);
    }

    pub fn damage_area(&mut self, caster: i32, p: u8, dmg: i32, rad: i32, x: i32, y: i32) {
        if dmg <= 0 {
            return;
        }
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(x, y, rad, &mut buf);
        for k in 0..n {
            let j = buf[k] as usize;
            if j as i32 == caster || self.alive[j] == 0 || !self.hostile(p, self.player[j]) {
                continue;
            }
            self.hurt_by(j, dmg, caster);
        }
        self.scratch = buf;
    }

    pub(crate) fn gather(&self, x: i32, y: i32, rad: i32, buf: &mut [u32]) -> usize {
        let rad = rad.max(0);
        let rcells = (fixed::round(rad) + 1).min(8);
        let rad2 = rad as i64 * rad as i64;
        let cx = fixed::to_int(x);
        let cy = fixed::to_int(y);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let mut count = 0usize;
        for oy in -rcells..=rcells {
            for ox in -rcells..=rcells {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let cell = (ny * w + nx) as usize;
                for ju in self.bucket_members(cell) {
                    if count == buf.len() {
                        return count;
                    }
                    let j = ju as u32;
                    if dist2(self.pos_x[ju], self.pos_y[ju], x, y) > rad2 {
                        continue;
                    }
                    buf[count] = j;
                    count += 1;
                }
            }
        }
        count
    }
}


