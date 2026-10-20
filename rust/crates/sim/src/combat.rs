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
/// Cells a unit's bucket scan reaches past its weapon range. A structure is bucketed at its
/// centre, which lies beyond its footprint edge, so the scan must reach past the edge to find it.
const STRUCTURE_SCAN_CELLS: i32 = 3;

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
/// One payable death, queued while a mod owns kill gold (`gold_by_mod`) and drained by the room, which
/// feeds it to the mod's `on_died` with these room ids. `killer` and `killer_player` are `None` for an
/// attackerless death (or a self-kill); `bounty` is the victim's bounty and is always positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payout {
    pub tick: u32,
    pub victim: u32,
    pub victim_player: u8,
    pub killer: Option<u32>,
    pub killer_player: Option<u8>,
    pub killer_hero: bool,
    pub bounty: i32,
}
/// Gold for the last hit on a hero: 125 + 8 per victim level (patch 7.31 notes; unchanged in later
/// notes). Assist gold is 15 + (50 + 0.037 * victim net worth) / assisting heroes (patch 7.40 notes).
const HERO_KILL_GOLD_BASE: i32 = 125;
const HERO_KILL_GOLD_PER_LEVEL: i32 = 8;
const ASSIST_GOLD_BASE: i64 = 15;
const ASSIST_GOLD_FLAT: i64 = 50;
const ASSIST_NET_WORTH_PER_MILLE: i64 = 37;

const HERO_BASE_MAGIC_RESIST_PCT: i64 = 25;

/// Armour reduction per armour point, Q16.16 (0.06): Dota's physical damage multiplier is
/// 1 - 0.06a / (1 + 0.06|a|), with no floor. Engine constant, not in the npc dump.
const ARMOUR_PER_POINT_Q16: i64 = 3932;

/// Weapon damage after armour, in the units `damage` is given in (whole units or Q16.16). `armour`
/// is Q16.16, as stored on every unit. Armour `a` scales damage by `1 - k*a / (1 + k*|a|)` with
/// `k` = ARMOUR_PER_POINT_Q16, so armour never reaches zero and negative armour raises damage.
/// Never below zero.
pub fn armour_reduced(damage: i32, armour: i32) -> i32 {
    let k = ARMOUR_PER_POINT_Q16;
    let ka = (k * armour as i64) >> fixed::SHIFT;
    let reduction = (ka << fixed::SHIFT) / (fixed::ONE as i64 + ka.abs());
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
            if self.alive[k] == 0 || (self.player[k] != p && self.vision[p as usize % MAX_PLAYERS] & (1 << self.player[k] as usize % MAX_PLAYERS) == 0) {
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
        if self.damage[i] <= 0 || self.stunned(i) || self.disarmed(i) {
            return;
        }
        if self.cooldown[i] > 0 {
            self.cooldown[i] -= 1;
            if self.cooldown[i] > 0 {
                return;
            }
        }
        let Some(b) = self.best_strike_target(i) else {
            self.aggro.strike[i] = -1;
            return;
        };
        self.aggro.strike[i] = b as i32;
        self.aggro.strike_gen[i] = self.aggro.spawn_gen[b];
        self.cooldown[i] = self.attack_period(i);
        if self.misses(i) {
            return;
        }
        self.strike(i, b);
    }

    /// True when unit `i` is on an attack-move order and an enemy is in weapon reach: it holds its
    /// position and strikes instead of walking past, and its route order resumes once the target
    /// dies or leaves reach.
    pub(crate) fn engaged(&self, i: usize) -> bool {
        self.order_kind[i] == ORDER_ATTACK_MOVE && self.best_strike_target(i).is_some()
    }

    /// The enemy unit auto-attack would strike for unit `i` this tick, if any: the taunt target when
    /// one is set, else the best visible enemy in weapon reach by target class, then distance.
    pub(crate) fn best_strike_target(&self, i: usize) -> Option<usize> {
        if self.damage[i] <= 0 || self.stunned(i) || self.disarmed(i) {
            return None;
        }
        let rc = (fixed::round(self.range_q[i]) + 1).clamp(1, 8) + STRUCTURE_SCAN_CELLS;
        let cx = cell_of(self.pos_x[i]);
        let cy = cell_of(self.pos_y[i]);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let mut best = -1i64;
        let mut best_class = u8::MAX;
        let reach_d2 = (self.range_sq[i] as i64) << fixed::SHIFT;
        let mut best_d2 = reach_d2;
        let current = self.sticky_strike(i);
        let mut current_class = None;
        for oy in -rc..=rc {
            for ox in -rc..=rc {
                let nx = cx + ox;
                let ny = cy + oy;
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let cell = (ny * w + nx) as usize;
                for j in self.bucket_members(cell) {
                    if j == i || self.alive[j] == 0 {
                        continue;
                    }
                    let d2 = self.strike_measure(i, j).1;
                    if d2 > reach_d2 {
                        continue;
                    }
                    if !self.hostile(self.player[i], self.player[j]) && !self.deniable(i, j) {
                        continue;
                    }
                    if (self.targets[i] & self.layer_of(j)) == 0 || !self.visible(self.player[i], j) || !self.targetable(self.player[i], j) {
                        continue;
                    }
                    let class = self.target_class(i, j);
                    if j as i32 == current {
                        current_class = Some(class);
                    }
                    if class < best_class || (class == best_class && d2 <= best_d2) {
                        best = j as i64;
                        best_class = class;
                        best_d2 = d2;
                    }
                }
            }
        }
        if let Some(class) = current_class {
            if class <= best_class {
                best = current as i64;
            }
        }
        if let Some(t) = self.taunt_target(i) {
            best = t as i64;
        }
        (best >= 0).then_some(best as usize)
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
        if self.stunned(i) {
            return Err(self.reject(Refusal::Stunned));
        }
        if self.disarmed(i) {
            return Err(self.reject(Refusal::Stunned));
        }
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        let in_reach = i != target
            && self.damage[i] > 0
            && (self.hostile(self.player[i], self.player[target]) || self.deniable(i, target))
            && (self.targets[i] & self.layer_of(target)) != 0
            && self.visible(self.player[i], target)
            && self.targetable(self.player[i], target)
            && self.strike_measure(i, target).1 <= reach;
        if !in_reach {
            return Err(self.reject(Refusal::InvalidOrderTarget));
        }
        if self.cooldown[i] > 1 {
            return Err(self.reject(Refusal::Cooldown));
        }
        self.cooldown[i] = self.attack_period(i);
        if self.misses(i) {
            return Ok(());
        }
        self.strike(i, target);
        Ok(())
    }

    /// Ability damage: the response and shield apply, armour does not.
    /// Ability damage to unit `j` with no attacker to credit, through the shared damage path.
    pub fn hurt(&mut self, j: usize, dmg: i32) {
        self.hurt_by(j, dmg, -1);
    }

    /// Ability damage after the magic resistance of unit `j`: `damage * (100 - resist) / 100`, the
    /// resist being the hero base of 25 percent (`npc_dota_hero_base` MagicalResistance; 0 for a
    /// non-hero). Ability damage is magical.
    fn magic_reduced(&self, j: usize, dmg: i32) -> i32 {
        let resist = if self.is_hero(j) { HERO_BASE_MAGIC_RESIST_PCT } else { 0 };
        (dmg as i64 * (100 - resist) / 100) as i32
    }

    /// Ability damage to unit `j` from unit `attacker` (-1 for none). An unknown unit is a no-op.
    pub(crate) fn hurt_by(&mut self, j: usize, dmg: i32, attacker: i32) {
        if j >= self.n {
            return;
        }
        let dmg = self.magic_reduced(j, dmg);
        self.hurt_typed(j, dmg, attacker);
    }

    /// Damage whose type was already applied (armour, magic resistance, pure): no further reduction.
    pub(crate) fn hurt_typed(&mut self, j: usize, dmg: i32, attacker: i32) {
        if j >= self.n {
            return;
        }
        let before = self.hp[j];
        self.inflict(j, dmg, attacker);
        self.stats.ability_damage = self.stats.ability_damage.wrapping_add(before - self.hp[j]);
    }

    /// The gold unit `j` is worth when destroyed: a hero's level-scaled bounty, or the catalog
    /// bounty of any other unit. Zero for an unknown unit.
    pub fn unit_bounty(&self, j: usize) -> i32 {
        if j >= self.n {
            return 0;
        }
        if self.is_hero(j) {
            HERO_KILL_GOLD_BASE + HERO_KILL_GOLD_PER_LEVEL * self.character_level(j) as i32
        } else {
            self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.bounty)
        }
    }

    /// Pays the destroyed unit's bounty. Any killer unit keeps it as its own gold, less the team
    /// share (see hero.rs). Only an attackerless kill pays the rival side's resource pool. Neither
    /// payout is made while a mod owns gold (`gold_by_mod`); the death is queued as a `Payout` for
    /// the mod instead, and the kill itself is unchanged.
    fn pay_bounty(&mut self, j: usize, attacker: i32) {
        let bounty = self.unit_bounty(j);
        if bounty <= 0 || self.was_denied(j, attacker) {
            return;
        }
        if self.gold_by_mod {
            self.queue_payout(j, attacker, bounty);
            return;
        }
        if attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j {
            self.hero_bounty(attacker as usize, bounty);
            return;
        }
        if let Some(rival) = self.rival_of(self.player[j]) {
            self.add_resources(rival, bounty, 0);
        }
    }

    fn queue_payout(&mut self, j: usize, attacker: i32, bounty: i32) {
        if self.payouts.len() >= KILL_QUEUE_CAP {
            return;
        }
        let killer = (attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j).then_some(attacker as usize);
        self.payouts.push(Payout {
            tick: self.ticks,
            victim: j as u32,
            victim_player: self.player[j],
            killer: killer.map(|k| k as u32),
            killer_player: killer.map(|k| self.player[k]),
            killer_hero: killer.is_some_and(|k| self.is_hero(k)),
            bounty,
        });
    }

    /// Takes the payable deaths queued since the last drain, oldest first.
    pub fn drain_payouts(&mut self) -> Vec<Payout> {
        self.payouts.drain(..).collect()
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
    pub(crate) fn inflict(&mut self, j: usize, mut dmg: i32, attacker: i32) {
        self.touch(j);
        if self.invulnerable(j) {
            dmg = 0;
        } else if self.is_illusion(j) {
            dmg = (dmg as i64 * self.spell.ill_in[j] as i64 / 100) as i32;
        }
        if dmg > 0 {
            let reduction = self.stat(j, crate::spell::S_DAMAGE_REDUCTION).clamp(-100, 100) as i64;
            if reduction != 0 {
                dmg = (dmg as i64 * (100 - reduction) / 100) as i32;
            }
            self.trig_hit[j] = 1;
            if attacker >= 0 && (attacker as usize) < self.n && attacker as usize != j {
                self.remember_hitter(j, attacker);
            }
            if self.spell.damage_interrupts {
                self.interrupt_channel(j);
            }
            self.last_hit[j] = self.ticks;
            match self.response[j] {
                RESPONSE_IMMUNE => dmg = 0,
                RESPONSE_REDUCED => dmg /= 2,
                _ => {}
            }
            dmg = self.mitigate_mech(j, dmg);
            dmg = self.absorb_shield(j, dmg);
            if attacker >= 0 && (attacker as usize) < self.n {
                self.aggro_on_hit(j, attacker as usize);
            }
        }
        if dmg >= self.hp[j] && self.status_left(j, crate::status::DEATH_PREVENTION) > 0 {
            dmg = (self.hp[j] - 1).max(0);
        }
        self.hp[j] = self.hp[j].wrapping_sub(dmg);
        if self.hp[j] <= 0 {
            let fell = self.alive[j] != 0;
            self.hp[j] = 0;
            self.alive[j] = 0;
            self.trig_death[j] = 1;
            self.stats.deaths += 1;
            if fell && !self.is_illusion(j) && !self.reincarnate(j) {
                self.record_death(j, attacker);
                self.pay_bounty(j, attacker);
                self.share_xp(j, attacker);
                self.hero_died(j);
                self.camp_died(j, attacker);
            }
        }
    }

    /// Books the death of unit `j`, and when `attacker` names another unit credits it a kill and
    /// queues the event for `drain_kill_events`.
    pub(crate) fn record_death(&mut self, j: usize, attacker: i32) {
        self.unit_deaths[j] = self.unit_deaths[j].saturating_add(1);
        let base = j * ASSIST_SLOTS;
        if attacker < 0 || attacker as usize >= self.n || attacker as usize == j {
            self.killer[j] = -1;
            self.hitter_id[base..base + ASSIST_SLOTS].fill(-1);
            return;
        }
        let k = attacker as usize;
        self.killer[j] = attacker;
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
        let mut helpers = [0usize; ASSIST_SLOTS];
        let mut helper_count = 0;
        for s in base..base + ASSIST_SLOTS {
            let id = self.hitter_id[s];
            self.hitter_id[s] = -1;
            if id < 0 || id as usize == killer || id as usize >= self.n || self.player[id as usize] != side {
                continue;
            }
            if self.ticks.wrapping_sub(self.hitter_tick[s]) <= ASSIST_WINDOW {
                self.unit_assists[id as usize] = self.unit_assists[id as usize].saturating_add(1);
                if self.is_hero(id as usize) && !helpers[..helper_count].contains(&(id as usize)) {
                    helpers[helper_count] = id as usize;
                    helper_count += 1;
                }
            }
        }
        if helper_count == 0 || !self.is_hero(j) || !self.hostile(self.player[killer], self.player[j]) {
            return;
        }
        let shared = (ASSIST_GOLD_FLAT * 1000 + self.hero_net_worth(j) as i64 * ASSIST_NET_WORTH_PER_MILLE) / 1000;
        let each = (ASSIST_GOLD_BASE + shared / helper_count as i64) as i32;
        for &u in &helpers[..helper_count] {
            self.heroes.gold[u] = self.heroes.gold[u].saturating_add(each);
        }
    }

    /// Gold held by hero `j` plus the cost of the items in its inventory and stash.
    pub(crate) fn hero_net_worth(&self, j: usize) -> i32 {
        let slots = j * crate::inventory::INV_SLOTS..(j + 1) * crate::inventory::INV_SLOTS;
        let items: i64 = slots
            .flat_map(|s| [(self.inv_kind[s], self.inv_count[s]), (self.stash_kind[s], self.stash_count[s])])
            .filter(|&(kind, count)| kind != 0 && count > 0)
            .map(|(kind, count)| self.cat.items.get(kind as usize - 1).map_or(0, |it| it.cost as i64) * count as i64)
            .sum();
        (self.hero_gold(j) as i64 + items).min(i32::MAX as i64) as i32
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

#[cfg(test)]
mod tests {
    use crate::{fixed, load, Sim};

    const CAP: usize = 1024;

    fn duel_sim() -> Sim {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let mut s = load(&image, CAP).expect("duel-routes sim");
        s.set_fog(false);
        s
    }

    /// A crowd past the inline cell capacity must stay fully bucketed, so no unit drops out of
    /// the strike and separation scans.
    #[test]
    fn crowded_cell_keeps_every_unit_bucketed() {
        let mut s = duel_sim();
        let crowd: Vec<usize> = (0..20).map(|_| creep(&mut s, 0)).collect();
        let (x, y) = (fixed::from_int(5), fixed::from_int(5));
        for &i in &crowd {
            assert!(s.set_position(i, x, y));
        }
        let cell = (5 * s.grid().w as usize) + 5;
        let mut members: Vec<usize> = s.bucket_members(cell).collect();
        members.sort_unstable();
        let mut expected = crowd.clone();
        expected.sort_unstable();
        assert_eq!(members, expected, "every unit in the crowded cell must be bucketed");
        let total: usize = (0..s.grid().w as usize * s.grid().h as usize).map(|c| s.bucket_members(c).count()).sum();
        let alive = (0..s.n).filter(|&i| s.alive(i)).count();
        assert_eq!(total, alive, "every live in-grid unit must be bucketed exactly once");
    }

    /// A creep for `player` on the first walkable cell the map has room for.
    fn creep(s: &mut Sim, player: u8) -> usize {
        let kind = s.catalog().unit_index("creep").expect("creep unit") as u32;
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find_map(|(x, y)| s.spawn_for(player, kind, fixed::from_int(x), fixed::from_int(y)))
            .expect("a free cell for the creep")
    }

    /// Kills one enemy creep by an allied unit's blow, then one by no attacker. Returns the
    /// killer's gold delta and the rival side's resource delta.
    fn kill_both_ways(s: &mut Sim) -> (i32, i32) {
        let killer = creep(s, 0);
        let gold_before = s.heroes.gold[killer];
        let victim = creep(s, 1);
        s.inflict(victim, fixed::from_int(10000), killer as i32);
        assert!(!s.alive(victim), "a lethal blow must still kill the unit");
        let killer_gold = s.heroes.gold[killer] - gold_before;

        let rival_before = s.resource_a(0);
        let neutral = creep(s, 1);
        s.inflict(neutral, fixed::from_int(10000), -1);
        assert!(!s.alive(neutral), "an attackerless lethal blow must still kill the unit");
        (killer_gold, s.resource_a(0) - rival_before)
    }

    #[test]
    fn kill_pays_the_usual_bounty_when_gold_is_not_owned_by_a_mod() {
        let mut s = duel_sim();
        assert!(!s.gold_by_mod(), "the flag defaults to off");
        let sample = creep(&mut s, 1);
        let bounty = s.unit_bounty(sample);
        assert!(bounty > 0, "the creep has a bounty");
        let share = bounty * s.heroes.rules.team_share_pct / 100;
        let (killer_gold, rival_gold) = kill_both_ways(&mut s);
        assert_eq!(killer_gold, bounty - share, "allied killer keeps the bounty less the team share");
        assert_eq!(rival_gold, bounty, "attackerless kill pays the rival side the bounty");
    }

    #[test]
    fn kill_pays_no_rust_bounty_when_gold_is_owned_by_a_mod() {
        let mut s = duel_sim();
        s.set_gold_by_mod(true);
        let (killer_gold, rival_gold) = kill_both_ways(&mut s);
        assert_eq!(killer_gold, 0, "no hero bounty while a mod owns gold");
        assert_eq!(rival_gold, 0, "no rival payout while a mod owns gold");

        let saved = s.save_state();
        let mut restored = duel_sim();
        restored.load_state(&saved).expect("save with the flag loads");
        assert!(restored.gold_by_mod(), "the flag survives a save and load");
    }

    /// A unit of catalog kind `name` for `player` on the first free walkable cell.
    fn unit_named(s: &mut Sim, player: u8, name: &str) -> usize {
        let kind = s.catalog().unit_index(name).expect("catalog unit") as u32;
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find_map(|(x, y)| s.spawn_for(player, kind, fixed::from_int(x), fixed::from_int(y)))
            .expect("a free cell for the unit")
    }

    /// A held unit keeps its tile while an enemy walks past it, and strikes an enemy inside its reach.
    #[test]
    fn held_unit_stays_put_and_strikes_only_in_reach() {
        let mut s = duel_sim();
        let holder = unit_named(&mut s, 0, "skirmisher");
        let (hx, hy) = s.position_fixed(holder);
        assert!(s.issue_order_kind(holder, crate::ORDER_HOLD, 0, 0).is_ok());

        let walker = creep(&mut s, 1);
        assert!(s.set_position(walker, hx - fixed::from_int(4), hy));
        assert!(s.issue_order_kind(walker, crate::ORDER_MOVE, hx + fixed::from_int(4), hy).is_ok());
        for _ in 0..200 {
            s.tick();
        }
        assert_eq!(s.position_fixed(holder), (hx, hy), "a held unit must not chase or be pushed off its tile");

        let rival = unit_named(&mut s, 1, "skirmisher");
        assert!(s.set_position(rival, hx, hy));
        let rival_hp = s.hp(rival);
        for _ in 0..50 {
            s.tick();
        }
        assert!(s.hp(rival) < rival_hp || !s.alive(rival), "a held unit must strike an enemy in its reach");
        assert_eq!(s.position_fixed(holder), (hx, hy), "striking from a hold must not move the unit");
    }

    /// First cell `(x, y)` of a walkable run `len` cells long on one row.
    fn open_run(s: &Sim, len: i32) -> Option<(i32, i32)> {
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        (0..h)
            .flat_map(|y| (0..w - len).map(move |x| (x, y)))
            .find(|&(x, y)| (0..len).all(|k| s.grid().walkable(x + k, y)))
    }

    /// Attack-move walks to its point through a line of enemies: it strikes each one met on the
    /// way, resumes travel after each kill, and stops at the point once no enemy is left.
    #[test]
    fn attack_move_reaches_point_through_enemies_and_stops_there() {
        let mut s = duel_sim();
        let (x0, y) = open_run(&s, 16).expect("a 16-cell walkable run");
        let hunter = unit_named(&mut s, 0, "skirmisher");
        assert!(s.set_position(hunter, fixed::from_int(x0), fixed::from_int(y)));
        let foes: Vec<usize> = [5, 8]
            .iter()
            .map(|&k| {
                let foe = creep(&mut s, 1);
                assert!(s.set_position(foe, fixed::from_int(x0 + k), fixed::from_int(y)));
                foe
            })
            .collect();
        let (gx, gy) = (fixed::from_int(x0 + 14), fixed::from_int(y));
        assert!(s.issue_order_kind(hunter, crate::ORDER_ATTACK_MOVE, gx, gy).is_ok());
        let mut arrived = false;
        for _ in 0..3000 {
            s.tick();
            let (px, py) = s.position_fixed(hunter);
            if (px - gx).abs() <= fixed::ONE && (py - gy).abs() <= fixed::ONE {
                arrived = true;
                break;
            }
        }
        assert!(arrived, "attack-move must travel to its point");
        assert!(s.alive(hunter), "the hunter must survive the route");
        for &foe in &foes {
            assert!(!s.alive(foe), "attack-move must kill each enemy it meets on the route");
        }
    }

    /// Using an item from an empty slot is refused and changes nothing.
    #[test]
    fn use_item_on_an_empty_slot_is_refused() {
        let mut s = duel_sim();
        let unit = creep(&mut s, 0);
        let before = s.hp(unit);
        assert!(!s.use_item(unit, 0, -1, 0, 0), "an empty slot has no item to use");
        assert_eq!(s.hp(unit), before, "a refused item use changes no unit state");
    }

    /// A consumable spends one charge per successful use, and a second use inside the item's cooldown is refused.
    #[test]
    fn consumable_use_spends_a_charge_and_refuses_reuse_in_cooldown() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("moba");
        let image = osdt_data::generate(&scenario).expect("moba image");
        let mut s = load(&image, CAP).expect("moba sim");
        s.set_fog(false);
        let hero = unit_named(&mut s, 0, "antimage");
        let (hx, hy) = s.position_fixed(hero);
        s.enroll_hero(hero, hx, hy);
        let item = crate::inventory::item_kind(s.catalog().item_index("faerie_fire").expect("faerie fire item"));
        assert!(s.inventory_add(hero, item, 2), "the hero takes two charges");
        let base = hero * crate::inventory::INV_SLOTS;
        let slot = (0..crate::inventory::MAIN_SLOTS).find(|&k| s.inv_kind[base + k] == item).expect("item slot");
        assert!(s.use_item(hero, slot, hero as i32, 0, 0), "the first use casts");
        assert_eq!(s.inv_count[base + slot], 1, "one charge is spent");
        assert!(!s.use_item(hero, slot, hero as i32, 0, 0), "a second use inside the cooldown is refused");
        assert_eq!(s.inv_count[base + slot], 1, "a refused use spends no charge");
    }
}
