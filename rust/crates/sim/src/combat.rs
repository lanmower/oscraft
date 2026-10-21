//! Weapon attacks and damage. A hit is reduced by the target's armour, then its
//! damage response, then absorbed by its shield, then taken from life. Ability
//! damage goes through the same path, minus the armour.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::*;

const RESPONSE_REDUCED: u8 = 1;
const RESPONSE_IMMUNE: u8 = 2;
/// Kill events held between drains. Kills past this in one drain window are still tallied per unit.
pub(crate) const KILL_QUEUE_CAP: usize = 256;
/// Recent hitters remembered per unit. A mod reads them on the unit's death (take_recent_hitters).
pub(crate) const HITTER_SLOTS: usize = 4;

/// One kill: unit `killer` (of `killer_player`) destroyed unit `victim` (of `victim_player`) at `tick`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KillEvent {
    pub tick: u32,
    pub killer: u32,
    pub killer_player: u8,
    pub victim: u32,
    pub victim_player: u8,
}
/// Weapon damage after armour, in the units `damage` is given in (whole units or Q16.16). `armour`
/// is Q16.16, as stored on every unit, and `k` the reduction per armour point in Q16.16 (the attr
/// rules' armour_per_point_q16, 0.06 in the moba rules). Armour `a` scales damage by
/// `1 - k*a / (1 + k*|a|)`, so armour never reaches zero and negative armour raises damage.
/// Never below zero.
pub fn armour_reduced(damage: i32, armour: i32, k: i32) -> i32 {
    let k = k as i64;
    let ka = (k * armour as i64) >> fixed::SHIFT;
    let reduction = (ka << fixed::SHIFT) / (fixed::ONE as i64 + ka.abs());
    let factor = fixed::ONE as i64 - reduction;
    ((damage as i64 * factor) >> fixed::SHIFT).clamp(0, i32::MAX as i64) as i32
}

impl Sim {
    /// The armour reduction per armour point (Q16.16) the attr rules set, the `k` of `armour_reduced`.
    pub(crate) fn armour_k(&self) -> i32 {
        self.attributed.rules.armour_per_point_q16
    }

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

    /// Whether unit `j` is hidden from enemies: cloaked.
    fn hidden(&self, j: usize) -> bool {
        self.cat.unit_ext.get(self.type_of[j] as usize).map_or(false, |e| e.cloak & 1 != 0)
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

    /// Auto-attack: strikes the target the mod commanded for this tick (`strike_target`) when it is still
    /// a living hostile enemy in weapon reach. Nothing commanded means no strike.
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
        self.cooldown[i] = self.attack_period(i);
        if self.misses(i) {
            return;
        }
        self.strike(i, b);
    }

    /// True when the mod holds attack-move unit `i` in place (`set_hold_override` set to true): it stays on
    /// its tile and strikes what the mod commanded. The pack decides the hold each tick; no hold is the default.
    pub(crate) fn engaged(&self, i: usize) -> bool {
        self.hold_override[i] == 1 && self.order_kind[i] == ORDER_ATTACK_MOVE
    }

    /// Whether a weapon hit can reduce unit `j`'s life: not physically immune, not invulnerable,
    /// and not fully immune to damage.
    pub fn strike_lands(&self, j: usize) -> bool {
        self.status_left(j, crate::status::PHYSICAL_IMMUNE) == 0 && !self.invulnerable(j) && self.response[j] != RESPONSE_IMMUNE
    }

    /// Whether unit `i` can hit unit `j` on the sim's own terms, ignoring reach: a living enemy of `i`
    /// (hostile by diplomacy) on a layer `i` hits, visible and targetable. The mod chooses the target.
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

    /// Whether unit `i` can strike unit `j` now: `can_target`, a unit that deals damage, and `j` in weapon reach.
    fn strike_ready(&self, i: usize, j: usize) -> bool {
        let reach = (self.range_sq[i] as i64) << fixed::SHIFT;
        self.damage[i] > 0 && self.can_target(i, j) && self.strike_measure(i, j).1 <= reach
    }

    /// Whether unit `i` may strike unit `j` this tick: not stunned or disarmed, and `strike_ready` (a weapon,
    /// a targetable living enemy in reach). The strike choice itself is the pack's; this is the sim's test of it.
    pub fn can_strike(&self, i: usize, j: usize) -> bool {
        !self.stunned(i) && !self.disarmed(i) && self.strike_ready(i, j)
    }

    /// Weapon damage and reach of unit `i`, fixed point: (damage, range). Damage is 0 for a unit with no weapon.
    pub fn weapon(&self, i: usize) -> (i32, i32) {
        (self.damage[i], self.range_q[i])
    }

    /// The enemy unit `i` is set to strike each tick, -1 for none (see `set_strike_target`).
    pub fn strike_of(&self, i: usize) -> i32 {
        self.strike_target[i]
    }

    /// The enemy unit `i` strikes this tick: the target the mod commanded (`strike_target`), when it is
    /// still `strike_ready`. A unit with no damage, stunned or disarmed strikes nothing.
    fn commanded_strike(&self, i: usize) -> Option<usize> {
        if self.damage[i] <= 0 || self.stunned(i) || self.disarmed(i) {
            return None;
        }
        let t = self.strike_target[i];
        if t < 0 || t as usize >= self.n {
            return None;
        }
        let j = t as usize;
        self.strike_ready(i, j).then_some(j)
    }

    /// Sets the enemy unit `i` strikes each tick: unit `target`, or -1 for none (any negative value is
    /// none). The mod sets it every tick; a target that dies, turns allied or leaves reach is not struck.
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
        if !self.strike_ready(i, target) {
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

    /// Ability damage after the magic resistance of unit `j` (`magic_resist_pct`, from its type data).
    /// Ability damage is magical.
    fn magic_reduced(&self, j: usize, dmg: i32) -> i32 {
        let resist = self.magic_resist_pct(j) as i64;
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

    /// The catalog bounty of unit `j` when destroyed. Zero for an unknown unit. The kill rule (mod code)
    /// reads it through `game.unit_bounty`; the engine pays nothing.
    pub fn unit_bounty(&self, j: usize) -> i32 {
        if j >= self.n {
            return 0;
        }
        self.cat.unit_ext.get(self.type_of[j] as usize).map_or(0, |e| e.bounty)
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
                RESPONSE_REDUCED => dmg = (dmg as i64 * self.attributed.rules.reduced_response_pct as i64 / 100) as i32,
                _ => {}
            }
            dmg = self.mitigate_mech(j, dmg);
            dmg = self.absorb_shield(j, dmg);
        }
        if dmg >= self.hp[j] && self.status_left(j, crate::status::DEATH_PREVENTION) > 0 {
            dmg = (self.hp[j] - 1).max(0);
        }
        let dealt = dmg.min(self.hp[j]).max(0);
        self.hp[j] = self.hp[j].wrapping_sub(dmg);
        self.log_hit(attacker, j, dealt);
        if self.hp[j] <= 0 {
            let fell = self.alive[j] != 0;
            self.hp[j] = 0;
            self.alive[j] = 0;
            self.trig_death[j] = 1;
            self.stats.deaths += 1;
            if fell && !self.is_illusion(j) && !self.reincarnate(j) {
                self.record_death(j, attacker);
                self.hold_dead(j);
                self.group_died(j);
            }
        }
    }

    /// Books the death of unit `j`: the death count, and the killer the on_died hook is fed (`attacker`, or -1).
    /// When `attacker` names another unit, the kill event is queued for `drain_kill_events`. The sim credits
    /// nothing: the kill tally and every reward are the pack's (game.add_kill, on_died).
    pub(crate) fn record_death(&mut self, j: usize, attacker: i32) {
        self.unit_deaths[j] = self.unit_deaths[j].saturating_add(1);
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

    /// Remembers that unit `hitter` hit unit `j` now, reusing its slot or the oldest slot.
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

    /// The recent hitters of unit `j` whose last hit is at most `window` ticks old, oldest slot order, and
    /// forgets all of them. A mod's death rule reads this on the victim's death (the assist credit).
    pub fn take_recent_hitters(&mut self, j: usize, window: u32) -> Vec<u32> {
        if j >= self.n {
            return Vec::new();
        }
        let base = j * HITTER_SLOTS;
        let mut out = Vec::new();
        for s in base..base + HITTER_SLOTS {
            let id = self.hitter_id[s];
            self.hitter_id[s] = -1;
            if id >= 0 && (id as usize) < self.n && self.ticks.wrapping_sub(self.hitter_tick[s]) <= window {
                out.push(id as u32);
            }
        }
        out
    }

    /// Adds `n` kills to unit `i`'s kill tally (saturating). Which kills count is the mod's rule (the pack counts hero kills);
    /// the engine books the death and the kill event for every killer, and attaches no meaning to the tally.
    pub fn add_kill(&mut self, i: usize, n: u32) {
        if i < self.n {
            self.unit_kills[i] = self.unit_kills[i].saturating_add(n);
        }
    }

    /// Adds `n` generic credits to unit `i` (saturating): a per-unit tally a mod's rule keeps, such as
    /// the assist count the post-game screen shows. The engine attaches no meaning to it.
    pub fn add_credit(&mut self, i: usize, n: u32) {
        if i < self.n {
            self.unit_credits[i] = self.unit_credits[i].saturating_add(n);
        }
    }

    /// The generic credits unit `i` holds (`add_credit`); zero for an unknown index.
    pub fn credits_of(&self, i: usize) -> u32 {
        self.unit_credits.get(i).copied().unwrap_or(0)
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

    /// Kills one enemy creep by an allied unit's blow, then one by no attacker. Returns the killer's
    /// gold delta and the rival side's resource delta. The engine pays neither: gold is the mod's.
    fn kill_both_ways(s: &mut Sim) -> (i32, i32) {
        let killer = creep(s, 0);
        let gold_before = s.attributed.gold[killer];
        let victim = creep(s, 1);
        s.inflict(victim, fixed::from_int(10000), killer as i32);
        assert!(!s.alive(victim), "a lethal blow must still kill the unit");
        let killer_gold = s.attributed.gold[killer] - gold_before;

        let rival_before = s.resource_a(0);
        let neutral = creep(s, 1);
        s.inflict(neutral, fixed::from_int(10000), -1);
        assert!(!s.alive(neutral), "an attackerless lethal blow must still kill the unit");
        (killer_gold, s.resource_a(0) - rival_before)
    }

    #[test]
    fn kill_pays_nothing_in_the_engine() {
        let mut s = duel_sim();
        let (killer_gold, rival_gold) = kill_both_ways(&mut s);
        assert_eq!(killer_gold, 0, "no engine gold for the killer");
        assert_eq!(rival_gold, 0, "no engine payout to the rival side");
    }

    /// The recent hitters of a victim are read once, in slot order, and a credit added to a hitter is kept on that unit.
    #[test]
    fn recent_hitters_are_read_once_and_credits_show_in_the_tally() {
        let mut s = duel_sim();
        let (hitter, other, victim) = (creep(&mut s, 0), creep(&mut s, 0), creep(&mut s, 1));
        s.inflict(victim, fixed::from_int(1), hitter as i32);
        s.inflict(victim, fixed::from_int(1), other as i32);
        assert_eq!(s.take_recent_hitters(victim, 50), vec![hitter as u32, other as u32]);
        assert!(s.take_recent_hitters(victim, 50).is_empty(), "the hitters are forgotten once read");
        s.add_credit(hitter, 2);
        s.add_credit(hitter, 1);
        assert_eq!(s.credits_of(hitter), 3, "the credits add up on the unit that holds them");
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
            assert!(s.set_strike_target(holder, rival as i32).is_ok());
            s.tick();
        }
        assert!(s.hp(rival) < rival_hp || !s.alive(rival), "a held unit must strike an enemy in its reach");
        assert_eq!(s.position_fixed(holder), (hx, hy), "striking from a hold must not move the unit");
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

    /// A consumable spends one charge per successful use, and a second use while the unit lacks energy is refused.
    #[test]
    fn consumable_use_spends_a_charge_and_refuses_reuse_in_cooldown() {
        let scenario = crate::test_pack_dir();
        let image = osdt_data::generate(&scenario).expect("pack image");
        let mut s = load(&image, CAP).expect("pack sim");
        s.set_fog(false);
        let attributed = unit_named(&mut s, 0, "antimage");
        let (hx, hy) = s.position_fixed(attributed);
        s.attribute_unit(attributed, hx, hy);
        let item = crate::inventory::item_kind(s.catalog().item_index("faerie_fire").expect("faerie fire item"));
        assert!(s.inventory_add(attributed, item, 2), "the unit takes two charges");
        let base = attributed * crate::inventory::INV_SLOTS;
        let slot = (0..crate::inventory::MAIN_SLOTS).find(|&k| s.inv_kind[base + k] == item).expect("item slot");
        assert!(s.use_item(attributed, slot, attributed as i32, 0, 0), "the first use casts");
        assert_eq!(s.inv_count[base + slot], 1, "one charge is spent");
        assert!(!s.use_item(attributed, slot, attributed as i32, 0, 0), "a second use inside the cooldown is refused");
        assert_eq!(s.stats.last_refusal, crate::rules::Refusal::Cooldown, "the second use is refused for the item cooldown");
        assert_eq!(s.inv_count[base + slot], 1, "a refused use spends no charge");
    }
}

#[cfg(test)]
mod structure_scan_tests {
    /// The structure scan the duel scenario's TUNE section carries: 3 cells past weapon range, as the
    /// constant was before the value moved into data.
    #[test]
    fn structure_scan_cells_loads_from_data_and_matches_old_constant() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let image = osdt_data::generate(&scenario).expect("duel-routes image");
        let s = crate::load(&image, 1024).expect("duel-routes sim");
        assert_eq!(s.catalog().tune.structure_scan_cells, 3);
    }
}
