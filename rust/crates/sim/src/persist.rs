//! Save and load of live sim state as bytes. A save holds every field the state hash covers,
//! plus the day and night clock, the mode flags and the counters, so a load followed by state_hash
//! gives the hash the save was taken at. Not saved: the catalog (the caller loads the same map
//! first) and the bucket planes and scratch (rebuilt by the tick). The flow-field pool is saved
//! in full, solver frontiers included, so a loaded run continues exactly as an uninterrupted one.
//! Layout is little-endian with fixed field order; this version is independent of the map format.

use crate::arrays::{Sim, Stats};
use crate::pathfinding::{Field, Grid, Pool};
use crate::rules::{Refusal, REFUSAL_KINDS};
use crate::*;

const MAGIC: &[u8; 4] = b"OSSV";
/// Version 16 adds the destroyed-trigger flags and the declared winner. Version 32 drops the team aura timers and adds per-unit healing. Older saves are refused.
/// Version 17 adds the item inventory and item cooldowns. Version 19 adds the stash bank. Older saves are refused.
/// Version 22 adds the day and night clock. Version 23 adds the status countdowns and slow multiplier.
/// Version 27 adds the attributed-unit state (respawn, gold, mana cap, spot) and the credit
/// tallies and recent hitters. Version 30 adds the per-slot spawn generation and the generation each sticky
/// strike target was struck under, so a reused slot never hands its old occupant's target to the new unit.
/// Version 31 adds the chase and return state of acquiring units (acquire.rs).
/// Version 33 drops the carrier and store-stock sections: a carrier's walk and a store's stock are mod
/// state now, and the sim keeps only the saved order point per unit (order_memory.rs). Saves of any other
/// version, older ones included, are refused with `BadVersion`.
/// Version 35 renames the sections to neutral names (attributed units, groups, dests, expiry,
/// item_requests, aura) and drops the skill-point rule from the engine: the layout is unchanged.
/// Version 38 moves the game clock to the pack (a night flag and the day cycle as plain storage), drops the reveal
/// list, the burrow state and the walk-cast pending slots, and adds the status flags, the stat bounds and the
/// per-unit strike target and research bits.
/// Version 37 adds the per-unit strike target and the research bits (the mod-owned research state).
/// Version 36 drops the kill-payout flag (the mod owns kill gold) and renames the assist tally to
/// the generic per-unit credits a mod's rule adds (`add_credit`).
/// Version 39 drops the respawn spot of a reincarnating hero (`spot_x`, `spot_y`): the aegis move keeps the spot in
/// the pack, so the layout after `mana_max` no longer carries those two arrays. Version 38 saves are refused.
/// Version 40 adds the four per-unit arrays the pack sets each tick after the strike target: the damage scale (the
/// armour curve moved to the pack), the speed floor and cap, and the miss chance. Version 39 saves are refused.
/// Version 41 adds the per-unit magical damage kept (the magic resistance moved to the pack). Version 40 saves are refused.
/// Version 42 adds the per-unit damage kept through the damage reduction stat and through the damage response (both moved
/// to the pack). Version 41 saves are refused.
// 43 adds a doodad's per-instance scale to the saved state.
const STATE_VERSION: u16 = 43;

/// Every refusal in discriminant order, so a saved code maps back to its variant.
const REFUSALS: [Refusal; REFUSAL_KINDS] = [
    Refusal::Unset,
    Refusal::NoSlot,
    Refusal::Cooldown,
    Refusal::Energy,
    Refusal::NoTarget,
    Refusal::UnseenTarget,
    Refusal::TargetFilter,
    Refusal::NoSpawnSlot,
    Refusal::NoTeleport,
    Refusal::NoMissileSlot,
    Refusal::EffectUnsupported,
    Refusal::CalldownNested,
    Refusal::NoHealerSlot,
    Refusal::MoveRange,
    Refusal::InvalidOrderKind,
    Refusal::InvalidOrderTarget,
    Refusal::CannotAfford,
    Refusal::NoTrain,
    Refusal::InvalidPlayer,
    Refusal::InvalidTeam,
    Refusal::InvalidCell,
    Refusal::ActionUnsupported,
    Refusal::CameraUnsupported,
    Refusal::SoundUnsupported,
    Refusal::UiPanelUnsupported,
    Refusal::ChatUnsupported,
    Refusal::UpgradeRefused,
    Refusal::CellFull,
    Refusal::CapacityBlocked,
    Refusal::Prerequisite,
    Refusal::ResearchBusy,
    Refusal::TransportFull,
    Refusal::Channelling,
    Refusal::OffGrid,
    Refusal::NoDoodad,
    Refusal::DuplicateDoodad,
    Refusal::TrainBusy,
    Refusal::NotLearned,
    Refusal::NoSkillTree,
    Refusal::NoSkillPoint,
    Refusal::SkillMaxed,
    Refusal::LevelGate,
    Refusal::NoItem,
    Refusal::OutOfReach,
    Refusal::InventoryFull,
    Refusal::MissingComponent,
    Refusal::Stunned,
    Refusal::Silenced,
    Refusal::Rooted,
    Refusal::Passive,
    Refusal::SpellImmune,
    Refusal::Muted,
];

/// Why a save could not be loaded. A failed load leaves the sim as it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    BadMagic,
    BadVersion(u16),
    /// The save was taken from a sim with another unit capacity or grid size.
    Mismatch,
    /// The bytes end before the state does, or continue after it.
    Length,
    BadRefusal(u8),
}

fn refusal(code: u8) -> Result<Refusal, StateError> {
    REFUSALS.get(code as usize).copied().ok_or(StateError::BadRefusal(code))
}

fn put_u32(w: &mut Vec<u8>, x: u32) {
    w.extend_from_slice(&x.to_le_bytes());
}

fn put_i32(w: &mut Vec<u8>, x: i32) {
    w.extend_from_slice(&x.to_le_bytes());
}

fn put_u64(w: &mut Vec<u8>, x: u64) {
    w.extend_from_slice(&x.to_le_bytes());
}

fn put_bool(w: &mut Vec<u8>, b: bool) {
    w.push(b as u8);
}

fn put_i32s(w: &mut Vec<u8>, v: &[i32]) {
    for x in v {
        w.extend_from_slice(&x.to_le_bytes());
    }
}

fn put_u32s(w: &mut Vec<u8>, v: &[u32]) {
    for x in v {
        w.extend_from_slice(&x.to_le_bytes());
    }
}

struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], StateError> {
        let end = self.at.checked_add(n).ok_or(StateError::Length)?;
        if end > self.buf.len() {
            return Err(StateError::Length);
        }
        let s = &self.buf[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, StateError> {
        Ok(self.take(1)?[0])
    }

    fn bool(&mut self) -> Result<bool, StateError> {
        Ok(self.u8()? != 0)
    }

    fn u16(&mut self) -> Result<u16, StateError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, StateError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32, StateError> {
        Ok(self.u32()? as i32)
    }

    fn u64(&mut self) -> Result<u64, StateError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    fn bytes_into(&mut self, v: &mut [u8]) -> Result<(), StateError> {
        let b = self.take(v.len())?;
        v.copy_from_slice(b);
        Ok(())
    }

    fn i32s_into(&mut self, v: &mut [i32]) -> Result<(), StateError> {
        let b = self.take(4 * v.len())?;
        for (x, c) in v.iter_mut().zip(b.chunks_exact(4)) {
            *x = i32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
        Ok(())
    }

    fn u32s_into(&mut self, v: &mut [u32]) -> Result<(), StateError> {
        let b = self.take(4 * v.len())?;
        for (x, c) in v.iter_mut().zip(b.chunks_exact(4)) {
            *x = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }
        Ok(())
    }
}

fn put_list(w: &mut Vec<u8>, v: &[u32]) {
    put_u64(w, v.len() as u64);
    put_u32s(w, v);
}

fn put_field(w: &mut Vec<u8>, f: &Field) {
    put_i32(w, f.goal_x);
    put_i32(w, f.goal_y);
    put_i32(w, f.want_x);
    put_i32(w, f.want_y);
    put_i32(w, f.b);
    put_bool(w, f.building);
    put_bool(w, f.ready);
    put_i32s(w, &f.dist);
    w.extend_from_slice(&f.dir);
    w.extend_from_slice(&f.dir_next);
    w.extend_from_slice(&f.settled);
    put_list(w, &f.cur);
    put_list(w, &f.n1);
    put_list(w, &f.n2);
}

/// The flow-field pool's saved bytes, for the state hash.
pub(crate) fn pool_bytes(p: &Pool, out: &mut Vec<u8>) {
    put_pool(out, p);
}

fn put_pool(w: &mut Vec<u8>, p: &Pool) {
    put_u64(w, p.owner.len() as u64);
    put_u64(w, p.round as u64);
    put_u32(w, p.tick_no);
    put_u64(w, p.budget as u64);
    put_i32s(w, &p.owner);
    put_i32s(w, &p.want_x);
    put_i32s(w, &p.want_y);
    for &d in &p.dirty {
        put_bool(w, d);
    }
    put_u32s(w, &p.age);
    put_u32s(w, &p.seen);
    for f in &p.fields {
        put_bool(w, f.is_some());
        if let Some(f) = f {
            put_field(w, f);
        }
    }
}

fn read_list(r: &mut Reader<'_>, v: &mut Vec<u32>) -> Result<(), StateError> {
    let n = r.u64()? as usize;
    let bytes = n.checked_mul(4).ok_or(StateError::Length)?;
    let b = r.take(bytes)?;
    v.clear();
    v.extend(b.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])));
    Ok(())
}

fn read_field(r: &mut Reader<'_>, w: usize, h: usize) -> Result<Field, StateError> {
    let mut f = Field::new(w, h);
    f.goal_x = r.i32()?;
    f.goal_y = r.i32()?;
    f.want_x = r.i32()?;
    f.want_y = r.i32()?;
    f.b = r.i32()?;
    f.building = r.bool()?;
    f.ready = r.bool()?;
    r.i32s_into(&mut f.dist)?;
    r.bytes_into(&mut f.dir)?;
    r.bytes_into(&mut f.dir_next)?;
    r.bytes_into(&mut f.settled)?;
    read_list(r, &mut f.cur)?;
    read_list(r, &mut f.n1)?;
    read_list(r, &mut f.n2)?;
    Ok(f)
}

fn read_pool(r: &mut Reader<'_>, p: &mut Pool) -> Result<(), StateError> {
    let cap = r.u64()? as usize;
    if cap != p.owner.len() {
        return Err(StateError::Mismatch);
    }
    p.round = r.u64()? as usize;
    p.tick_no = r.u32()?;
    p.budget = r.u64()? as usize;
    r.i32s_into(&mut p.owner)?;
    r.i32s_into(&mut p.want_x)?;
    r.i32s_into(&mut p.want_y)?;
    for k in 0..cap {
        p.dirty[k] = r.bool()?;
    }
    r.u32s_into(&mut p.age)?;
    r.u32s_into(&mut p.seen)?;
    for k in 0..cap {
        p.fields[k] = if r.bool()? { Some(read_field(r, p.w, p.h)?) } else { None };
    }
    Ok(())
}

impl Sim {
    /// Serialises the live state. The sim is only read.
    pub fn save_state(&self) -> Vec<u8> {
        let mut w = Vec::new();
        w.extend_from_slice(MAGIC);
        w.extend_from_slice(&STATE_VERSION.to_le_bytes());
        put_u32(&mut w, self.cap as u32);
        put_u32(&mut w, self.grid.w as u32);
        put_u32(&mut w, self.grid.h as u32);

        put_u64(&mut w, self.n as u64);
        put_u64(&mut w, self.head as u64);
        put_u32(&mut w, self.ticks);
        put_bool(&mut w, self.night);
        put_u32s(&mut w, &self.day_cycle);
        put_u64(&mut w, self.rng);
        put_bool(&mut w, self.fog);
        put_bool(&mut w, self.abil_on);

        let s = &self.stats;
        put_u64(&mut w, s.deaths as u64);
        put_u64(&mut w, s.vision_refreshes as u64);
        put_u64(&mut w, s.casts as u64);
        put_u64(&mut w, s.casts_refused as u64);
        for c in s.refused_by {
            put_u64(&mut w, c as u64);
        }
        w.push(s.last_refusal as u8);
        put_u64(&mut w, s.buffs_dropped as u64);
        put_u64(&mut w, s.summoned as u64);
        put_u64(&mut w, s.missiles_lost as u64);
        put_i32(&mut w, s.ability_damage);

        w.extend_from_slice(&self.grid.flags);
        w.extend_from_slice(&self.grid.tiles);

        put_i32s(&mut w, &self.pos_x);
        put_i32s(&mut w, &self.pos_y);
        put_i32s(&mut w, &self.vel_x);
        put_i32s(&mut w, &self.vel_y);
        put_i32s(&mut w, &self.tgt_x);
        put_i32s(&mut w, &self.tgt_y);
        put_i32s(&mut w, &self.hp);
        put_i32s(&mut w, &self.hp_max);
        put_i32s(&mut w, &self.radius);
        put_i32s(&mut w, &self.speed);
        put_i32s(&mut w, &self.damage);
        put_i32s(&mut w, &self.armor);
        put_i32s(&mut w, &self.range_q);
        put_i32s(&mut w, &self.range_sq);
        put_i32s(&mut w, &self.period);
        put_i32s(&mut w, &self.cooldown);
        put_i32s(&mut w, &self.shield);
        put_i32s(&mut w, &self.shield_max);
        put_i32s(&mut w, &self.sight_cells);
        w.extend_from_slice(&self.player);
        w.extend_from_slice(&self.alive);
        put_i32s(&mut w, &self.type_of);
        put_u32s(&mut w, &self.filter);
        w.extend_from_slice(&self.response);
        w.extend_from_slice(&self.flying);
        w.extend_from_slice(&self.targets);
        w.extend_from_slice(&self.kind);
        w.extend_from_slice(&self.blocked);
        w.extend_from_slice(&self.routed);
        w.extend_from_slice(&self.order_kind);
        w.extend_from_slice(&self.groups.member);
        put_i32s(&mut w, &self.groups.anchor_x);
        put_i32s(&mut w, &self.groups.anchor_y);
        w.extend_from_slice(&self.groups.waiting);
        put_u32s(&mut w, &self.groups.revive_at);
        put_i32s(&mut w, &self.patrol_x);
        put_i32s(&mut w, &self.patrol_y);
        put_i32s(&mut w, &self.amount);
        put_i32s(&mut w, &self.resource_a);
        put_i32s(&mut w, &self.resource_b);
        put_i32s(&mut w, &self.abil_idx);
        put_i32s(&mut w, &self.abil_cd);
        w.extend_from_slice(&self.abil_rank);
        put_i32s(&mut w, &self.energy);
        put_u32s(&mut w, &self.changed_at);
        w.extend_from_slice(&self.trig_hit);
        w.extend_from_slice(&self.trig_spawn);
        w.extend_from_slice(&self.trig_idle);
        w.extend_from_slice(&self.trig_death);
        w.extend_from_slice(&self.trig_ready);
        put_u64(&mut w, self.winner as i64 as u64);
        put_i32s(&mut w, &self.buff_kind);
        put_i32s(&mut w, &self.buff_left);
        w.extend_from_slice(&self.aura_player);
        put_i32s(&mut w, &self.aura_left);
        put_i32s(&mut w, &self.aura_timer);
        put_i32s(&mut w, &self.aura_period);
        put_i32s(&mut w, &self.aura_amt);
        put_i32s(&mut w, &self.aura_rad);
        put_i32s(&mut w, &self.aura_x);
        put_i32s(&mut w, &self.aura_y);
        put_i32s(&mut w, &self.aura_target);
        w.extend_from_slice(&self.m_alive);
        put_i32s(&mut w, &self.m_timer);
        put_i32s(&mut w, &self.m_caster);
        put_i32s(&mut w, &self.m_ability);
        put_i32s(&mut w, &self.m_target);
        put_i32s(&mut w, &self.m_x);
        put_i32s(&mut w, &self.m_y);
        w.extend_from_slice(&self.vis);
        w.extend_from_slice(&self.seen);
        put_i32s(&mut w, &self.seen_x);
        put_i32s(&mut w, &self.seen_y);
        put_i32s(&mut w, &self.channel_left);
        put_i32s(&mut w, &self.channel_ai);
        put_i32s(&mut w, &self.channel_x);
        put_i32s(&mut w, &self.channel_y);
        put_i32s(&mut w, &self.channel_target);
        put_i32s(&mut w, &self.status_left);
        put_i32s(&mut w, &self.slow_mul);

        for &b in &self.has_seer {
            put_bool(&mut w, b);
        }
        w.extend_from_slice(&self.team);
        for a in self.alliance {
            w.extend_from_slice(&a.to_le_bytes());
        }
        for v in self.vision {
            w.extend_from_slice(&v.to_le_bytes());
        }
        w.extend_from_slice(&self.controller);
        for &t in &self.last_hit {
            put_u32(&mut w, t);
        }
        put_u64(&mut w, self.doodads.len() as u64);
        for d in &self.doodads {
            put_u32(&mut w, d.id);
            w.extend_from_slice(&d.kind.to_le_bytes());
            put_i32(&mut w, d.x);
            put_i32(&mut w, d.y);
            put_i32(&mut w, d.rotation);
            put_i32(&mut w, d.scale);
        }
        put_u64(&mut w, self.log_len as u64);
        put_u64(&mut w, self.log_dropped);
        for k in 0..self.log_len {
            put_u32(&mut w, self.log_tick[k]);
            put_i32(&mut w, self.log_unit[k]);
            w.push(self.log_kind[k]);
            put_i32(&mut w, self.log_x[k]);
            put_i32(&mut w, self.log_y[k]);
        }
        put_pool(&mut w, &self.paths);
        put_u32s(&mut w, &self.inv_kind);
        put_u32s(&mut w, &self.inv_count);
        put_u32s(&mut w, &self.stash_kind);
        put_u32s(&mut w, &self.stash_count);
        put_i32s(&mut w, &self.item_cd);
        put_u32s(&mut w, &self.xp_total);
        put_u32s(&mut w, &self.unit_kills);
        put_u32s(&mut w, &self.unit_deaths);
        w.extend_from_slice(&self.attributed.is_attributed);
        put_i32s(&mut w, &self.attributed.home_x);
        put_i32s(&mut w, &self.attributed.home_y);
        w.extend_from_slice(&self.attributed.waiting);
        put_u32s(&mut w, &self.attributed.respawn_at);
        put_i32s(&mut w, &self.attributed.gold);
        put_i32s(&mut w, &self.attributed.mana_max);
        put_u32s(&mut w, &self.unit_credits);
        put_i32s(&mut w, &self.hitter_id);
        put_u32s(&mut w, &self.hitter_tick);
        put_i32s(&mut w, &self.strike_target);
        put_i32s(&mut w, &self.armour_scale);
        put_i32s(&mut w, &self.speed_floor);
        put_i32s(&mut w, &self.speed_cap);
        put_i32s(&mut w, &self.miss_chance);
        put_i32s(&mut w, &self.magic_keep);
        put_i32s(&mut w, &self.damage_keep);
        put_i32s(&mut w, &self.response_keep);
        w.extend_from_slice(&self.researched);
        w.extend_from_slice(&self.dests.id);
        w.extend_from_slice(&self.dests.step);
        put_i32s(&mut w, &self.aggro.target);
        put_u32s(&mut w, &self.aggro.until);
        put_u32s(&mut w, &self.aggro.invuln_until);
        w.extend_from_slice(&self.aggro.reduce_pct);
        put_i32s(&mut w, &self.aggro.heal);
        put_i32s(&mut w, &self.chase.target);
        w.extend_from_slice(&self.chase.ret_kind);
        put_i32s(&mut w, &self.chase.ret_x);
        put_i32s(&mut w, &self.chase.ret_y);
        put_u32s(&mut w, &self.expiry.expires);
        w.extend_from_slice(&self.order_memory.set);
        put_i32s(&mut w, &self.order_memory.x);
        put_i32s(&mut w, &self.order_memory.y);
        for table in self.spell.tables() {
            put_i32s(&mut w, table);
        }
        w
    }

    /// Replaces the live state with a save taken from a sim with the same unit capacity and
    /// grid size (the same map). The catalog is kept. On any error the sim is unchanged.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), StateError> {
        let mut r = Reader { buf: bytes, at: 0 };
        if r.take(4)? != &MAGIC[..] {
            return Err(StateError::BadMagic);
        }
        let version = r.u16()?;
        if version != STATE_VERSION {
            return Err(StateError::BadVersion(version));
        }
        let cap = r.u32()? as usize;
        let (w, h) = (r.u32()? as usize, r.u32()? as usize);
        if cap != self.cap || w != self.grid.w || h != self.grid.h {
            return Err(StateError::Mismatch);
        }

        let mut next = Sim::with_catalog(self.cap, Grid::open(w, h), self.cat.clone());
        next.xp_curve = self.xp_curve.clone();
        next.attributed = crate::attributes::AttributedState::new(cap);
        next.attributed.rules = self.attributed.rules;
        next.attributed.defs = self.attributed.defs.clone();
        next.n = r.u64()? as usize;
        next.head = r.u64()? as usize;
        next.ticks = r.u32()?;
        next.night = r.bool()?;
        r.u32s_into(&mut next.day_cycle)?;
        next.rng = r.u64()?;
        next.fog = r.bool()?;
        next.abil_on = r.bool()?;

        let mut st = Stats::default();
        st.deaths = r.u64()? as usize;
        st.vision_refreshes = r.u64()? as usize;
        st.casts = r.u64()? as usize;
        st.casts_refused = r.u64()? as usize;
        for k in 0..REFUSAL_KINDS {
            st.refused_by[k] = r.u64()? as usize;
        }
        st.last_refusal = refusal(r.u8()?)?;
        st.buffs_dropped = r.u64()? as usize;
        st.summoned = r.u64()? as usize;
        st.missiles_lost = r.u64()? as usize;
        st.ability_damage = r.i32()?;
        next.stats = st;

        r.bytes_into(&mut next.grid.flags)?;
        r.bytes_into(&mut next.grid.tiles)?;
        next.grid.heights = self.grid.heights.clone();
        next.grid.height_step = self.grid.height_step;
        next.cat_digest = self.cat_digest;

        r.i32s_into(&mut next.pos_x)?;
        r.i32s_into(&mut next.pos_y)?;
        r.i32s_into(&mut next.vel_x)?;
        r.i32s_into(&mut next.vel_y)?;
        r.i32s_into(&mut next.tgt_x)?;
        r.i32s_into(&mut next.tgt_y)?;
        r.i32s_into(&mut next.hp)?;
        r.i32s_into(&mut next.hp_max)?;
        r.i32s_into(&mut next.radius)?;
        r.i32s_into(&mut next.speed)?;
        r.i32s_into(&mut next.damage)?;
        r.i32s_into(&mut next.armor)?;
        r.i32s_into(&mut next.range_q)?;
        r.i32s_into(&mut next.range_sq)?;
        r.i32s_into(&mut next.period)?;
        r.i32s_into(&mut next.cooldown)?;
        r.i32s_into(&mut next.shield)?;
        r.i32s_into(&mut next.shield_max)?;
        r.i32s_into(&mut next.sight_cells)?;
        r.bytes_into(&mut next.player)?;
        r.bytes_into(&mut next.alive)?;
        r.i32s_into(&mut next.type_of)?;
        r.u32s_into(&mut next.filter)?;
        r.bytes_into(&mut next.response)?;
        r.bytes_into(&mut next.flying)?;
        r.bytes_into(&mut next.targets)?;
        r.bytes_into(&mut next.kind)?;
        r.bytes_into(&mut next.blocked)?;
        r.bytes_into(&mut next.routed)?;
        r.bytes_into(&mut next.order_kind)?;
        r.bytes_into(&mut next.groups.member)?;
        r.i32s_into(&mut next.groups.anchor_x)?;
        r.i32s_into(&mut next.groups.anchor_y)?;
        r.bytes_into(&mut next.groups.waiting)?;
        r.u32s_into(&mut next.groups.revive_at)?;
        r.i32s_into(&mut next.patrol_x)?;
        r.i32s_into(&mut next.patrol_y)?;
        r.i32s_into(&mut next.amount)?;
        r.i32s_into(&mut next.resource_a)?;
        r.i32s_into(&mut next.resource_b)?;
        r.i32s_into(&mut next.abil_idx)?;
        r.i32s_into(&mut next.abil_cd)?;
        r.bytes_into(&mut next.abil_rank)?;
        r.i32s_into(&mut next.energy)?;
        r.u32s_into(&mut next.changed_at)?;
        r.bytes_into(&mut next.trig_hit)?;
        r.bytes_into(&mut next.trig_spawn)?;
        r.bytes_into(&mut next.trig_idle)?;
        r.bytes_into(&mut next.trig_death)?;
        r.bytes_into(&mut next.trig_ready)?;
        next.winner = r.u64()? as i64 as i32;
        r.i32s_into(&mut next.buff_kind)?;
        r.i32s_into(&mut next.buff_left)?;
        r.bytes_into(&mut next.aura_player)?;
        r.i32s_into(&mut next.aura_left)?;
        r.i32s_into(&mut next.aura_timer)?;
        r.i32s_into(&mut next.aura_period)?;
        r.i32s_into(&mut next.aura_amt)?;
        r.i32s_into(&mut next.aura_rad)?;
        r.i32s_into(&mut next.aura_x)?;
        r.i32s_into(&mut next.aura_y)?;
        r.i32s_into(&mut next.aura_target)?;
        r.bytes_into(&mut next.m_alive)?;
        r.i32s_into(&mut next.m_timer)?;
        r.i32s_into(&mut next.m_caster)?;
        r.i32s_into(&mut next.m_ability)?;
        r.i32s_into(&mut next.m_target)?;
        r.i32s_into(&mut next.m_x)?;
        r.i32s_into(&mut next.m_y)?;
        r.bytes_into(&mut next.vis)?;
        r.bytes_into(&mut next.seen)?;
        r.i32s_into(&mut next.seen_x)?;
        r.i32s_into(&mut next.seen_y)?;
        r.i32s_into(&mut next.channel_left)?;
        r.i32s_into(&mut next.channel_ai)?;
        r.i32s_into(&mut next.channel_x)?;
        r.i32s_into(&mut next.channel_y)?;
        r.i32s_into(&mut next.channel_target)?;
        r.i32s_into(&mut next.status_left)?;
        r.i32s_into(&mut next.slow_mul)?;

        for k in 0..MAX_PLAYERS {
            next.has_seer[k] = r.bool()?;
        }
        r.bytes_into(&mut next.team)?;
        for k in 0..MAX_PLAYERS {
            next.alliance[k] = r.u16()?;
        }
        for k in 0..MAX_PLAYERS {
            next.vision[k] = r.u16()?;
        }
        crate::fog::bump_vision_generation();
        r.bytes_into(&mut next.controller)?;

        r.u32s_into(&mut next.last_hit)?;
        let doodad_count = r.u64()? as usize;
        if doodad_count > bytes.len() / 22 {
            return Err(StateError::Length);
        }
        next.doodads = Vec::with_capacity(doodad_count);
        for _ in 0..doodad_count {
            let id = r.u32()?;
            let kind = r.u16()?;
            let x = r.i32()?;
            let y = r.i32()?;
            let rotation = r.i32()?;
            let scale = r.i32()?;
            next.doodads.push(crate::loader::Doodad { id, kind, x, y, rotation, scale });
        }
        let log_len = r.u64()? as usize;
        if log_len > ORDER_LOG_CAP {
            return Err(StateError::Length);
        }
        next.log_len = log_len;
        next.log_dropped = r.u64()?;
        for k in 0..log_len {
            next.log_tick[k] = r.u32()?;
            next.log_unit[k] = r.i32()?;
            next.log_kind[k] = r.u8()?;
            next.log_x[k] = r.i32()?;
            next.log_y[k] = r.i32()?;
        }
        read_pool(&mut r, &mut next.paths)?;
        r.u32s_into(&mut next.inv_kind)?;
        r.u32s_into(&mut next.inv_count)?;
        r.u32s_into(&mut next.stash_kind)?;
        r.u32s_into(&mut next.stash_count)?;
        r.i32s_into(&mut next.item_cd)?;
        r.u32s_into(&mut next.xp_total)?;
        r.u32s_into(&mut next.unit_kills)?;
        r.u32s_into(&mut next.unit_deaths)?;
        r.bytes_into(&mut next.attributed.is_attributed)?;
        r.i32s_into(&mut next.attributed.home_x)?;
        r.i32s_into(&mut next.attributed.home_y)?;
        r.bytes_into(&mut next.attributed.waiting)?;
        r.u32s_into(&mut next.attributed.respawn_at)?;
        r.i32s_into(&mut next.attributed.gold)?;
        r.i32s_into(&mut next.attributed.mana_max)?;
        r.u32s_into(&mut next.unit_credits)?;
        r.i32s_into(&mut next.hitter_id)?;
        r.u32s_into(&mut next.hitter_tick)?;
        r.i32s_into(&mut next.strike_target)?;
        r.i32s_into(&mut next.armour_scale)?;
        r.i32s_into(&mut next.speed_floor)?;
        r.i32s_into(&mut next.speed_cap)?;
        r.i32s_into(&mut next.miss_chance)?;
        r.i32s_into(&mut next.magic_keep)?;
        r.i32s_into(&mut next.damage_keep)?;
        r.i32s_into(&mut next.response_keep)?;
        r.bytes_into(&mut next.researched)?;
        r.bytes_into(&mut next.dests.id)?;
        r.bytes_into(&mut next.dests.step)?;
        r.i32s_into(&mut next.aggro.target)?;
        r.u32s_into(&mut next.aggro.until)?;
        r.u32s_into(&mut next.aggro.invuln_until)?;
        r.bytes_into(&mut next.aggro.reduce_pct)?;
        r.i32s_into(&mut next.aggro.heal)?;
        r.i32s_into(&mut next.chase.target)?;
        r.bytes_into(&mut next.chase.ret_kind)?;
        r.i32s_into(&mut next.chase.ret_x)?;
        r.i32s_into(&mut next.chase.ret_y)?;
        r.u32s_into(&mut next.expiry.expires)?;
        {
            r.bytes_into(&mut next.order_memory.set)?;
            r.i32s_into(&mut next.order_memory.x)?;
            r.i32s_into(&mut next.order_memory.y)?;
        }
        for table in next.spell.tables_mut() {
            r.i32s_into(table)?;
        }
        if r.at != bytes.len() {
            return Err(StateError::Length);
        }
        next.rebucket();
        next.resync_unit_mana();
        *self = next;
        Ok(())
    }
}
