//! Save and load of live sim state as bytes. A save holds every field the state hash covers,
//! plus the fog reveals, the mode flags and the counters, so a load followed by state_hash
//! gives the hash the save was taken at. Not saved: the catalog (the caller loads the same map
//! first) and the bucket planes and scratch (rebuilt by the tick). The flow-field pool is saved
//! in full, solver frontiers included, so a loaded run continues exactly as an uninterrupted one.
//! Layout is little-endian with fixed field order; this version is independent of the map format.

use crate::arrays::{Reveal, Sim, Stats};
use crate::pathfinding::{Field, Grid, Pool};
use crate::rules::{Refusal, REFUSAL_KINDS};
use crate::*;

const MAGIC: &[u8; 4] = b"OSSV";
/// Version 16 adds the destroyed-trigger flags and the declared winner. Older saves are refused.
/// Version 17 adds the item inventory and item cooldowns. Version 19 adds the stash bank. Older saves are refused.
/// Version 22 adds the day and night clock. Version 23 adds the status countdowns and slow multiplier.
/// Version 27 adds the hero per-unit state (respawn, buyback, gold, mana cap, spot) and the assist
/// tallies and recent hitters.
const STATE_VERSION: u16 = 28;

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
    Refusal::SkillLevelGate,
    Refusal::NoItem,
    Refusal::NoShop,
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
        put_u32(&mut w, self.day_clock);
        put_u64(&mut w, self.rng);
        put_i32(&mut w, self.scan_left);
        put_bool(&mut w, self.fog);
        put_bool(&mut w, self.harvest);
        put_bool(&mut w, self.produce);
        put_bool(&mut w, self.abil_on);
        put_bool(&mut w, self.gold_by_mod);

        let s = &self.stats;
        put_u64(&mut w, s.deaths as u64);
        put_u64(&mut w, s.vision_refreshes as u64);
        put_u64(&mut w, s.harvested as u64);
        put_u64(&mut w, s.trained as u64);
        put_u64(&mut w, s.trains_refused as u64);
        w.push(s.last_train_refusal as u8);
        put_u64(&mut w, s.retargets as u64);
        put_u64(&mut w, s.retargets_deferred as u64);
        put_u64(&mut w, s.casts as u64);
        put_u64(&mut w, s.casts_refused as u64);
        for c in s.refused_by {
            put_u64(&mut w, c as u64);
        }
        w.push(s.last_refusal as u8);
        put_u64(&mut w, s.buffs_dropped as u64);
        put_u64(&mut w, s.summoned as u64);
        put_u64(&mut w, s.reveals_dropped as u64);
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
        put_i32s(&mut w, &self.product);
        w.extend_from_slice(&self.flying);
        w.extend_from_slice(&self.targets);
        w.extend_from_slice(&self.kind);
        w.extend_from_slice(&self.blocked);
        w.extend_from_slice(&self.routed);
        w.extend_from_slice(&self.order_kind);
        w.extend_from_slice(&self.camps.is_camp);
        put_i32s(&mut w, &self.camps.home_x);
        put_i32s(&mut w, &self.camps.home_y);
        w.extend_from_slice(&self.camps.waiting);
        put_u32s(&mut w, &self.camps.respawn_at);
        put_i32s(&mut w, &self.patrol_x);
        put_i32s(&mut w, &self.patrol_y);
        put_i32s(&mut w, &self.amount);
        w.extend_from_slice(&self.wstate);
        put_i32s(&mut w, &self.wtarget);
        put_i32s(&mut w, &self.wtimer);
        put_i32s(&mut w, &self.carry);
        w.extend_from_slice(&self.carry_kind);
        put_i32s(&mut w, &self.harvest_amt);
        put_i32s(&mut w, &self.trainee);
        put_i32s(&mut w, &self.train_left);
        w.extend_from_slice(&self.train_hold);
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
        w.extend_from_slice(&self.healer_player);
        put_i32s(&mut w, &self.healer_left);
        put_i32s(&mut w, &self.healer_timer);
        put_i32s(&mut w, &self.healer_period);
        put_i32s(&mut w, &self.healer_amt);
        put_i32s(&mut w, &self.healer_rad);
        put_i32s(&mut w, &self.healer_x);
        put_i32s(&mut w, &self.healer_y);
        put_i32s(&mut w, &self.healer_target);
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
        w.extend_from_slice(&self.researched);
        put_i32s(&mut w, &self.research_left);
        put_i32s(&mut w, &self.research_rec);
        put_i32s(&mut w, &self.loaded_into);
        put_i32s(&mut w, &self.channel_left);
        put_i32s(&mut w, &self.channel_ai);
        put_i32s(&mut w, &self.channel_x);
        put_i32s(&mut w, &self.channel_y);
        put_i32s(&mut w, &self.channel_target);
        w.extend_from_slice(&self.burrowed);
        put_i32s(&mut w, &self.burrow_left);
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
        for r in &self.reveals {
            w.push(r.p);
            put_i32(&mut w, r.x0);
            put_i32(&mut w, r.y0);
            put_i32(&mut w, r.x1);
            put_i32(&mut w, r.y1);
            put_i32(&mut w, r.until);
            put_bool(&mut w, r.live);
        }
        for &t in &self.last_hit {
            put_u32(&mut w, t);
        }
        w.extend_from_slice(&self.upg_done);
        put_u64(&mut w, self.doodads.len() as u64);
        for d in &self.doodads {
            put_u32(&mut w, d.id);
            w.extend_from_slice(&d.kind.to_le_bytes());
            put_i32(&mut w, d.x);
            put_i32(&mut w, d.y);
            put_i32(&mut w, d.rotation);
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
        w.extend_from_slice(&self.heroes.is_hero);
        put_i32s(&mut w, &self.heroes.home_x);
        put_i32s(&mut w, &self.heroes.home_y);
        w.extend_from_slice(&self.heroes.waiting);
        put_u32s(&mut w, &self.heroes.respawn_at);
        put_u32s(&mut w, &self.heroes.buyback_ready);
        put_i32s(&mut w, &self.heroes.gold);
        put_i32s(&mut w, &self.heroes.mana_max);
        put_i32s(&mut w, &self.heroes.spot_x);
        put_i32s(&mut w, &self.heroes.spot_y);
        put_u32s(&mut w, &self.unit_assists);
        put_i32s(&mut w, &self.hitter_id);
        put_u32s(&mut w, &self.hitter_tick);
        w.extend_from_slice(&self.routes.lane);
        w.extend_from_slice(&self.routes.step);
        put_i32s(&mut w, &self.aggro.target);
        put_u32s(&mut w, &self.aggro.until);
        put_u32s(&mut w, &self.aggro.creep_seen);
        put_u32s(&mut w, &self.aggro.glyph_until);
        put_u32s(&mut w, &self.aggro.glyph_ready);
        put_u32s(&mut w, &self.aggro.glyph_need);
        put_u32s(&mut w, &self.aggro.glyph_skip);
        put_u32s(&mut w, &self.wards.expires);
        w.extend_from_slice(&self.couriers.phase);
        put_i32s(&mut w, &self.couriers.target);
        put_i32s(&mut w, &self.couriers.reach);
        put_i32s(&mut w, &self.couriers.home_x);
        put_i32s(&mut w, &self.couriers.home_y);
        w.extend_from_slice(&self.stock.count);
        put_u32s(&mut w, &self.stock.next_at);
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
        next.heroes = crate::hero::HeroState::new(cap);
        next.heroes.rules = self.heroes.rules;
        next.heroes.defs = self.heroes.defs.clone();
        next.n = r.u64()? as usize;
        next.head = r.u64()? as usize;
        next.ticks = r.u32()?;
        next.day_clock = r.u32()? % (DAY_TICKS + NIGHT_TICKS);
        next.rng = r.u64()?;
        next.scan_left = r.i32()?;
        next.fog = r.bool()?;
        next.harvest = r.bool()?;
        next.produce = r.bool()?;
        next.abil_on = r.bool()?;
        next.gold_by_mod = r.bool()?;

        let mut st = Stats::default();
        st.deaths = r.u64()? as usize;
        st.vision_refreshes = r.u64()? as usize;
        st.harvested = r.u64()? as usize;
        st.trained = r.u64()? as usize;
        st.trains_refused = r.u64()? as usize;
        st.last_train_refusal = refusal(r.u8()?)?;
        st.retargets = r.u64()? as usize;
        st.retargets_deferred = r.u64()? as usize;
        st.casts = r.u64()? as usize;
        st.casts_refused = r.u64()? as usize;
        for k in 0..REFUSAL_KINDS {
            st.refused_by[k] = r.u64()? as usize;
        }
        st.last_refusal = refusal(r.u8()?)?;
        st.buffs_dropped = r.u64()? as usize;
        st.summoned = r.u64()? as usize;
        st.reveals_dropped = r.u64()? as usize;
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
        r.i32s_into(&mut next.product)?;
        r.bytes_into(&mut next.flying)?;
        r.bytes_into(&mut next.targets)?;
        r.bytes_into(&mut next.kind)?;
        r.bytes_into(&mut next.blocked)?;
        r.bytes_into(&mut next.routed)?;
        r.bytes_into(&mut next.order_kind)?;
        r.bytes_into(&mut next.camps.is_camp)?;
        r.i32s_into(&mut next.camps.home_x)?;
        r.i32s_into(&mut next.camps.home_y)?;
        r.bytes_into(&mut next.camps.waiting)?;
        r.u32s_into(&mut next.camps.respawn_at)?;
        r.i32s_into(&mut next.patrol_x)?;
        r.i32s_into(&mut next.patrol_y)?;
        r.i32s_into(&mut next.amount)?;
        r.bytes_into(&mut next.wstate)?;
        r.i32s_into(&mut next.wtarget)?;
        r.i32s_into(&mut next.wtimer)?;
        r.i32s_into(&mut next.carry)?;
        r.bytes_into(&mut next.carry_kind)?;
        r.i32s_into(&mut next.harvest_amt)?;
        r.i32s_into(&mut next.trainee)?;
        r.i32s_into(&mut next.train_left)?;
        r.bytes_into(&mut next.train_hold)?;
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
        r.bytes_into(&mut next.healer_player)?;
        r.i32s_into(&mut next.healer_left)?;
        r.i32s_into(&mut next.healer_timer)?;
        r.i32s_into(&mut next.healer_period)?;
        r.i32s_into(&mut next.healer_amt)?;
        r.i32s_into(&mut next.healer_rad)?;
        r.i32s_into(&mut next.healer_x)?;
        r.i32s_into(&mut next.healer_y)?;
        r.i32s_into(&mut next.healer_target)?;
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
        r.bytes_into(&mut next.researched)?;
        r.i32s_into(&mut next.research_left)?;
        r.i32s_into(&mut next.research_rec)?;
        r.i32s_into(&mut next.loaded_into)?;
        r.i32s_into(&mut next.channel_left)?;
        r.i32s_into(&mut next.channel_ai)?;
        r.i32s_into(&mut next.channel_x)?;
        r.i32s_into(&mut next.channel_y)?;
        r.i32s_into(&mut next.channel_target)?;
        r.bytes_into(&mut next.burrowed)?;
        r.i32s_into(&mut next.burrow_left)?;
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
        r.bytes_into(&mut next.controller)?;
        for k in 0..REVEAL_CAP {
            next.reveals[k] = Reveal {
                p: r.u8()?,
                x0: r.i32()?,
                y0: r.i32()?,
                x1: r.i32()?,
                y1: r.i32()?,
                until: r.i32()?,
                live: r.bool()?,
            };
        }

        r.u32s_into(&mut next.last_hit)?;
        r.bytes_into(&mut next.upg_done)?;
        let doodad_count = r.u64()? as usize;
        if doodad_count > bytes.len() / 18 {
            return Err(StateError::Length);
        }
        next.doodads = Vec::with_capacity(doodad_count);
        for _ in 0..doodad_count {
            let id = r.u32()?;
            let kind = r.u16()?;
            let x = r.i32()?;
            let y = r.i32()?;
            let rotation = r.i32()?;
            next.doodads.push(crate::loader::Doodad { id, kind, x, y, rotation });
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
        r.bytes_into(&mut next.heroes.is_hero)?;
        r.i32s_into(&mut next.heroes.home_x)?;
        r.i32s_into(&mut next.heroes.home_y)?;
        r.bytes_into(&mut next.heroes.waiting)?;
        r.u32s_into(&mut next.heroes.respawn_at)?;
        r.u32s_into(&mut next.heroes.buyback_ready)?;
        r.i32s_into(&mut next.heroes.gold)?;
        r.i32s_into(&mut next.heroes.mana_max)?;
        r.i32s_into(&mut next.heroes.spot_x)?;
        r.i32s_into(&mut next.heroes.spot_y)?;
        r.u32s_into(&mut next.unit_assists)?;
        r.i32s_into(&mut next.hitter_id)?;
        r.u32s_into(&mut next.hitter_tick)?;
        r.bytes_into(&mut next.routes.lane)?;
        r.bytes_into(&mut next.routes.step)?;
        r.i32s_into(&mut next.aggro.target)?;
        r.u32s_into(&mut next.aggro.until)?;
        r.u32s_into(&mut next.aggro.creep_seen)?;
        r.u32s_into(&mut next.aggro.glyph_until)?;
        r.u32s_into(&mut next.aggro.glyph_ready)?;
        r.u32s_into(&mut next.aggro.glyph_need)?;
        r.u32s_into(&mut next.aggro.glyph_skip)?;
        r.u32s_into(&mut next.wards.expires)?;
        r.bytes_into(&mut next.couriers.phase)?;
        r.i32s_into(&mut next.couriers.target)?;
        r.i32s_into(&mut next.couriers.reach)?;
        r.i32s_into(&mut next.couriers.home_x)?;
        r.i32s_into(&mut next.couriers.home_y)?;
        r.bytes_into(&mut next.stock.count)?;
        r.u32s_into(&mut next.stock.next_at)?;
        for table in next.spell.tables_mut() {
            r.i32s_into(table)?;
        }
        if r.at != bytes.len() {
            return Err(StateError::Length);
        }
        next.rebucket();
        next.resync_hero_mana();
        *self = next;
        Ok(())
    }
}
