//! Weapon attacks and damage. A hit is reduced by the target's armour, then its
//! damage response, then absorbed by its shield, then taken from life. Ability
//! damage goes through the same path, minus the armour.

use crate::arrays::{dist2, cell_of, Sim};
use crate::fixed;
use crate::*;

const RESPONSE_REDUCED: u8 = 1;
const RESPONSE_IMMUNE: u8 = 2;
/// Kill events held between drains. Kills past this in one drain window are still tallied per unit.
pub(crate) const KILL_QUEUE_CAP: usize = 256;
/// Recent hitters remembered per unit for assist credit.
pub(crate) const ASSIST_SLOTS: usize = 4;
/// Ticks after a hit within which the hitter is credited an assist on the victim's death.
pub(crate) const ASSIST_WINDOW: u32 = 50;

/// One player's post-game tally, summed over the player's units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerTally {
    pub player: u8,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub gold: i32,
    pub xp: u32,
    /// Item kinds held (non-empty inventory slots), in unit then slot order.
    pub items: Vec<u32>,
}

/// One kill: unit `killer` (of `killer_player`) destroyed unit `victim` (of `victim_player`) at `tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KillEvent {
    pub tick: u32,
    pub killer: u32,
    pub killer_player: u8,
    pub victim: u32,
    pub victim_player: u8,
}
/// The least a weapon hit can do after armour, so armour never zeroes a hit (Q16.16, one quarter).
const ARMOUR_FLOOR: i32 = fixed::ONE / 4;

/// Armour reduction per armour point, Q16.16 (0.06). UNVERIFIED: the standard Dota armour
/// constant, recalled rather than checked against a source; confirm before relying on it.
const ARMOUR_PER_POINT_Q16: i64 = 3932;

/// Weapon damage after armour, in the units both arguments share (whole units or Q16.16). Armour
/// `a` scales damage by `1 - k*a / (1 + k*|a|)` with `k` = ARMOUR_PER_POINT_Q16, so armour never
/// reaches zero and negative armour raises damage. Never below zero. The attack paths apply
/// ARMOUR_FLOOR after this.
pub fn armour_reduced(damage: i32, armour: i32) -> i32 {
    let a = armour as i64;
    let k = ARMOUR_PER_POINT_Q16;
    let reduction = ((k * a) << fixed::SHIFT) / (fixed::ONE as i64 + k * a.abs());
    let factor = fixed::ONE as i64 - reduction;
    ((damage as i64 * factor) >> fixed::SHIFT).clamp(0, i32::MAX as i64) as i32
}

impl Sim {
    /// Final damage factor (Q16) for a weapon hit by unit `attacker` on unit `target`: the DMGF
    /// record for the attacker's weapon damage type and the target's size category, or 1.0 when
    /// no record applies.
    pub(crate) fn final_factor(&self, attacker: usize, target: usize) -> i32 {
        let dtype = self.cat.unit_ext.get(self.type_of[attacker] as usize).map_or(0, |e| e.damage_type);
        let size = self.cat.unit_ext.get(self.type_of[target] as usize).map_or(0, |e| e.size);
        self.cat
            .damage_factors
            .iter()
            .find(|f| f.damage_type == dtype && f.size == size)
            .map_or(fixed::ONE, |f| f.factor)
    }

    /// A weapon hit after armour, scaled by the final damage factor of the attacker's type and
    /// the target's size category.
    pub fn scale_hit(&self, dmg: i32, attacker: usize, target: usize) -> i32 {
        ((dmg as i64 * self.final_factor(attacker, target) as i64) >> fixed::SHIFT) as i32
    }

    /// Whether a ranged shot by unit `i` misses. A shot is ranged when its weapon reaches past
    /// melee range. The chance is the UEXT miss chance (Q16), rolled on the sim's RNG, which advances
    /// once per ranged shot. The format has no high-ground or cover rule, so the chance applies to
    /// every ranged shot.
    pub fn misses(&mut self, i: usize) -> bool {
        let chance = self.cat.unit_ext.get(self.type_of[i] as usize).map_or(0, |e| e.miss_chance);
        if chance <= 0 || self.range_q[i] <= MELEE_RANGE {
            return false;
        }
        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let roll = ((self.rng >> 33) & 0xFFFF) as i32;
        roll < chance
    }

    /// Whether unit `j` is hidden from enemies: cloaked, or burrowed.
    fn hidden(&self, j: usize) -> bool {
        let cloaked = self.cat.unit_ext.get(self.type_of[j] as usize).map_or(false, |e| e.cloak & 1 != 0);
        cloaked || self.burrowed[j] != 0
    }

    /// Whether one of player `p`'s own units with a detector radius reaches unit `j`.
    pub fn detected(&self, p: u8, j: usize) -> bool {
        (0..self.n).any(|k| {
            if self.alive[k] == 0 || self.player[k] != p {
                return false;
            }
            let radius = self.cat.unit_ext.get(self.type_of[k] as usize).map_or(0, |e| e.detector_radius);
            if radius <= 0 {
                return false;
            }
            let reach = fixed::from_milli(radius) as i64;
            dist2(self.pos_x[k], self.pos_y[k], self.pos_x[j], self.pos_y[j]) <= reach * reach
        })
    }

    /// Whether player `p` may target unit `j`: a hidden unit needs a detector covering it.
    pub fn targetable(&self, p: u8, j: usize) -> bool {
        !self.hidden(j) || self.detected(p, j)
    }

    /// Starts unit `i` burrowing for its UEXT burrow time; a zero time burrows at once. Only a
    /// unit with the can-burrow flag may burrow. Refusals change nothing: a dead or unknown unit
    /// is NoTarget; a unit that cannot burrow, or is already burrowed or burrowing, is InvalidOrderTarget.
    pub fn burrow_unit(&mut self, i: usize) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let ext = self.cat.unit_ext.get(self.type_of[i] as usize).copied().unwrap_or_default();
        if ext.cloak & 2 == 0 || self.burrowed[i] != 0 || self.burrow_left[i] > 0 {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if ext.burrow_ticks <= 0 {
            self.burrowed[i] = 1;
        } else {
            self.burrow_left[i] = ext.burrow_ticks;
        }
        self.touch(i);
        Ok(())
    }

    /// Counts down a burrow in progress on unit `i`; the unit is burrowed when it ends.
    pub(crate) fn burrow_step(&mut self, i: usize) {
        if self.burrow_left[i] > 0 {
            self.burrow_left[i] -= 1;
            if self.burrow_left[i] == 0 {
                self.burrowed[i] = 1;
                self.touch(i);
            }
        }
    }

    /// Auto-attack: nearest visible enemy on a layer this weapon may hit, within range.
    pub(crate) fn attack(&mut self, i: usize) {
        if self.damage[i] <= 0 {
            return;
        }
        if self.cooldown[i] > 0 {
            self.cooldown[i] -= 1;
            return;
        }
        let rc = (fixed::round(self.range_q[i]) + 1).clamp(1, 8);
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let mut best = -1i64;
        let mut best_d2 = (self.range_sq[i] as i64) << fixed::SHIFT;
        for oy in -rc..=rc {
            for ox in -rc..=rc {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let cell = (ny * w + nx) as usize;
                let cnt = self.bucket_n[cell] as usize;
                for k in 0..cnt {
                    let j = self.bucket_idx[cell * BUCKET_CAP + k] as usize;
                    if j == i || self.alive[j] == 0 || (!self.hostile(self.player[i], self.player[j]) && !self.deniable(i, j)) {
                        continue;
                    }
                    if (self.targets[i] & self.layer_of(j)) == 0 || !self.visible(self.player[i], j) || !self.targetable(self.player[i], j) {
                        continue;
                    }
                    let d2 = dist2(self.pos_x[i], self.pos_y[i], self.pos_x[j], self.pos_y[j]);
                    if d2 <= best_d2 {
                        best = j as i64;
                        best_d2 = d2;
                    }
                }
            }
        }
        if best < 0 {
            return;
        }
        let b = best as usize;
        let dmg = self.scale_hit(armour_reduced(self.damage[i], self.armor[b]).max(ARMOUR_FLOOR), i, b);
        self.cooldown[i] = self.period[i];
        if self.misses(i) {
            return;
        }
        self.inflict(b, dmg, i as i32);
    }

    /// Orders unit `i` to strike unit `target` through the weapon path (cooldown, armour,
    /// damage response, shield, then life). Refusals, each changing nothing: a dead or unknown
    /// unit is NoTarget; a target that is not a living visible enemy in weapon reach, or an
    /// attacker that cannot hit, is InvalidOrderTarget; a weapon on cooldown is Cooldown.
    pub fn order_attack(&mut self, i: usize, target: usize) -> Result<(), crate::rules::Refusal> {
        use crate::rules::Refusal;
        if i >= self.n || self.alive[i] == 0 || target >= self.n || self.alive[target] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        let in_reach = i != target
            && self.damage[i] > 0
            && (self.hostile(self.player[i], self.player[target]) || self.deniable(i, target))
            && (self.targets[i] & self.layer_of(target)) != 0
            && self.visible(self.player[i], target)
            && self.targetable(self.player[i], target)
            && dist2(self.pos_x[i], self.pos_y[i], self.pos_x[target], self.pos_y[target]) <= reach;
        if !in_reach {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.cooldown[i] > 0 {
            return Err(self.reject(Refusal::Cooldown));
        }
        let dmg = self.scale_hit(armour_reduced(self.damage[i], self.armor[target]).max(ARMOUR_FLOOR), i, target);
        self.cooldown[i] = self.period[i];
        if self.misses(i) {
            return Ok(());
        }
        self.inflict(target, dmg, i as i32);
        Ok(())
    }

    /// Ability damage: the response and shield apply, armour does not.
    /// Ability damage to unit `j` with no attacker to credit, through the shared damage path.
    pub fn hurt(&mut self, j: usize, dmg: i32) {
        self.hurt_by(j, dmg, -1);
    }

    /// Ability damage to unit `j` from unit `attacker` (-1 for none). An unknown unit is a no-op.
    pub(crate) fn hurt_by(&mut self, j: usize, dmg: i32, attacker: i32) {
        if j >= self.n {
            return;
        }
        let before = self.hp[j];
        self.inflict(j, dmg, attacker);
        self.stats.ability_damage = self.stats.ability_damage.wrapping_add(before - self.hp[j]);
    }

    /// Pays the destroyed unit's bounty. A hero killer keeps it as gold, less the team share (see
    /// hero.rs); any other killer pays the rival side's resource pool.
    fn pay_bounty(&mut self, j: usize, attacker: i32) {
        let bounty = self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.bounty);
        if bounty <= 0 || self.was_denied(j, attacker) {
            return;
        }
        if attacker >= 0 && self.is_hero(attacker as usize) {
            self.hero_bounty(attacker as usize, bounty);
            return;
        }
        if let Some(rival) = self.rival_of(self.player[j]) {
            self.add_resources(rival, bounty, 0);
        }
    }

    /// Whether unit `i` may deny unit `j`: `j` is allied to `i` (not `i` itself) and its health is at
    /// or below its deny threshold, `deny_pct` percent of max health. A zero threshold never denies.
    pub(crate) fn deniable(&self, i: usize, j: usize) -> bool {
        if i == j || self.alive[j] == 0 || self.hostile(self.player[i], self.player[j]) {
            return false;
        }
        let pct = self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.deny_pct as i64);
        pct > 0 && self.hp[j] as i64 * 100 <= self.hp_max[j] as i64 * pct
    }

    /// Whether the death of unit `j` to `attacker` was a denial: an allied unit landed the blow.
    fn was_denied(&self, j: usize, attacker: i32) -> bool {
        attacker >= 0
            && (attacker as usize) < self.n
            && attacker as usize != j
            && !self.hostile(self.player[attacker as usize], self.player[j])
    }

    /// Shares the xp of unit `j` killed by `attacker`. An enemy kill gives `xp_bounty` to the killer
    /// and to each hero allied to it within the victim's assist radius, split evenly with any
    /// remainder to the killer. A denial gives `xp_bounty * deny_xp_pct / 100` to each enemy hero
    /// within the radius, split the same way. The radius is the victim's assist radius; "in range"
    /// means inside it, not that the hero hit the victim.
    fn share_xp(&mut self, j: usize, attacker: i32) {
        if attacker < 0 || attacker as usize >= self.n || attacker as usize == j {
            return;
        }
        let k = attacker as usize;
        let ext = self.cat.unit_ext.get(self.type_of[j] as usize).copied().unwrap_or_default();
        let denied = self.was_denied(j, attacker);
        let pool = if denied {
            (ext.xp_bounty.max(0) as i64 * ext.deny_xp_pct as i64 / 100) as u32
        } else {
            ext.xp_bounty.max(0) as u32
        };
        if pool == 0 {
            return;
        }
        let (victim_side, killer_side) = (self.player[j], self.player[k]);
        let mut recipients = Vec::new();
        if !denied {
            recipients.push(k);
        }
        let radius = fixed::from_milli(ext.assist_radius);
        let mut buf = std::mem::take(&mut self.scratch);
        let n = self.gather(self.pos_x[j], self.pos_y[j], radius, &mut buf);
        for &u in &buf[..n] {
            let u = u as usize;
            if u == j || u == k || self.alive[u] == 0 || !self.is_hero(u) {
                continue;
            }
            let on_side = if denied {
                self.hostile(self.player[u], victim_side)
            } else {
                !self.hostile(self.player[u], killer_side)
            };
            if on_side {
                recipients.push(u);
            }
        }
        self.scratch = buf;
        if recipients.is_empty() {
            return;
        }
        let count = recipients.len() as u32;
        let share = pool / count;
        let rest = pool - share * count;
        for (idx, &u) in recipients.iter().enumerate() {
            self.add_xp(u, share + if idx == 0 { rest } else { 0 });
        }
    }

    /// The damage path shared by weapons and abilities: damage response, then shield,
    /// then life, with death recorded when life runs out. `attacker` is the unit dealing the
    /// damage, or -1; a death it causes is credited to it.
    fn inflict(&mut self, j: usize, mut dmg: i32, attacker: i32) {
        self.touch(j);
        if dmg > 0 {
            self.trig_hit[j] = 1;
            if attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j {
                self.remember_hitter(j, attacker);
            }
            self.interrupt_channel(j);
            self.last_hit[j] = self.ticks;
            match self.response[j] {
                RESPONSE_IMMUNE => dmg = 0,
                RESPONSE_REDUCED => dmg /= 2,
                _ => {}
            }
            dmg = self.absorb_shield(j, dmg);
        }
        self.hp[j] = self.hp[j].wrapping_sub(dmg);
        if self.hp[j] <= 0 {
            let fell = self.alive[j] != 0;
            self.hp[j] = 0;
            self.alive[j] = 0;
            self.trig_death[j] = 1;
            self.stats.deaths += 1;
            if fell {
                self.record_death(j, attacker);
                self.pay_bounty(j, attacker);
                self.share_xp(j, attacker);
                self.hero_died(j);
            }
        }
    }

    /// Books the death of unit `j`, and when `attacker` names another unit credits it a kill and
    /// queues the event for `drain_kill_events`.
    pub(crate) fn record_death(&mut self, j: usize, attacker: i32) {
        self.unit_deaths[j] = self.unit_deaths[j].saturating_add(1);
        let base = j * ASSIST_SLOTS;
        if attacker < 0 || attacker as usize >= self.n || attacker as usize == j {
            self.hitter_id[base..base + ASSIST_SLOTS].fill(-1);
            return;
        }
        let k = attacker as usize;
        self.unit_kills[k] = self.unit_kills[k].saturating_add(1);
        self.credit_assists(j, k);
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

    /// Remembers that unit `hitter` hit unit `j` now, reusing its slot or the oldest slot.
    fn remember_hitter(&mut self, j: usize, hitter: i32) {
        let base = j * ASSIST_SLOTS;
        let slots = base..base + ASSIST_SLOTS;
        let slot = slots
            .clone()
            .find(|&s| self.hitter_id[s] == hitter)
            .or_else(|| slots.clone().find(|&s| self.hitter_id[s] < 0))
            .unwrap_or_else(|| slots.min_by_key(|&s| self.hitter_tick[s]).unwrap_or(base));
        self.hitter_id[slot] = hitter;
        self.hitter_tick[slot] = self.ticks;
    }

    /// Credits an assist to each recent hitter of dead unit `j` that shares the killer's side and
    /// is not the killer, then forgets the hitters.
    fn credit_assists(&mut self, j: usize, killer: usize) {
        let base = j * ASSIST_SLOTS;
        let side = self.player[killer];
        for s in base..base + ASSIST_SLOTS {
            let id = self.hitter_id[s];
            self.hitter_id[s] = -1;
            if id < 0 || id as usize == killer || id as usize >= self.n || self.player[id as usize] != side {
                continue;
            }
            if self.ticks.wrapping_sub(self.hitter_tick[s]) <= ASSIST_WINDOW {
                self.unit_assists[id as usize] = self.unit_assists[id as usize].saturating_add(1);
            }
        }
    }

    /// Each player's post-game tally: kills, deaths, assists, gold, xp and held items, summed over
    /// that player's unit slots. Dead units keep their tallies until their slot is respawned.
    pub fn post_game(&self) -> Vec<PlayerTally> {
        (0..self.player_count())
            .map(|p| {
                let player = p as u8;
                let mut tally = PlayerTally {
                    player,
                    kills: 0,
                    deaths: 0,
                    assists: 0,
                    gold: self.resource_a(player),
                    xp: 0,
                    items: Vec::new(),
                };
                for i in (0..self.n).filter(|&i| self.player[i] == player) {
                    tally.kills = tally.kills.saturating_add(self.unit_kills[i]);
                    tally.deaths = tally.deaths.saturating_add(self.unit_deaths[i]);
                    tally.assists = tally.assists.saturating_add(self.unit_assists[i]);
                    tally.xp = tally.xp.saturating_add(self.xp_total[i]);
                    let slots = i * crate::inventory::INV_SLOTS;
                    tally.items.extend(
                        (slots..slots + crate::inventory::INV_SLOTS)
                            .filter(|&s| self.inv_kind[s] != 0 && self.inv_count[s] > 0)
                            .map(|s| self.inv_kind[s]),
                    );
                }
                tally
            })
            .collect()
    }

    /// Takes the kill events queued since the last drain, oldest first. The wasm layer drains once per tick.
    pub fn drain_kill_events(&mut self) -> Vec<KillEvent> {
        self.kill_queue.drain(..).collect()
    }

    /// Damage to the cast target unit alone, for a damage effect with radius 0. Only a living
    /// enemy other than the caster takes it.
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

    /// Damages every living enemy unit within `rad` of a point. `caster` is the unit that
    /// cast it, or -1; the caster is spared.
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

    /// Writes the indices of units inside a disc into `buf`, in cell then slot order.
    /// Returns how many were written; `buf` is never grown.
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
                let cnt = self.bucket_n[cell] as usize;
                for k in 0..cnt {
                    if count == buf.len() {
                        return count;
                    }
                    let j = self.bucket_idx[cell * BUCKET_CAP + k];
                    let ju = j as usize;
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
