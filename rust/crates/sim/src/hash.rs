//! FNV-1a over the struct-of-arrays bytes, in a fixed field order. Two runs that
//! agree on this agree on every unit, resource, buff, projectile, fog plane and counter.

use crate::arrays::Sim;
use crate::loader::Catalog;
use crate::pathfinding::Grid;

struct Fnv(u64);

impl Fnv {
    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 = (self.0 ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    fn i32s(&mut self, v: &[i32]) {
        for x in v {
            self.bytes(&x.to_le_bytes());
        }
    }
    fn u32s(&mut self, v: &[u32]) {
        for x in v {
            self.bytes(&x.to_le_bytes());
        }
    }
    fn u64s(&mut self, v: &[u64]) {
        for x in v {
            self.bytes(&x.to_le_bytes());
        }
    }
}

/// Digest of the mod: every parsed catalog record (through its Debug form), the grid's height
/// levels and step. Computed once per load, so a state hash changes when the mod does.
pub(crate) fn catalog_digest(cat: &Catalog, grid: &Grid) -> u64 {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    h.bytes(format!("{:?}", cat).as_bytes());
    h.bytes(&grid.heights);
    h.i32s(&[grid.height_step]);
    h.0
}

impl Sim {
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        h.u64s(&[self.cat_digest, self.cap as u64]);
        h.bytes(&[self.abil_on as u8, self.fog as u8, self.harvest as u8, self.produce as u8]);
        let mut pool = Vec::new();
        crate::persist::pool_bytes(&self.paths, &mut pool);
        h.bytes(&pool);
        h.u64s(&[self.n as u64, self.head as u64, self.ticks as u64, self.day_clock as u64, self.rng, self.scan_left as u64]);
        h.u64s(&[self.log_len as u64, self.log_dropped]);
        h.u32s(&self.log_tick[..self.log_len]);
        h.i32s(&self.log_unit[..self.log_len]);
        h.bytes(&self.log_kind[..self.log_len]);
        h.i32s(&self.log_x[..self.log_len]);
        h.i32s(&self.log_y[..self.log_len]);
        h.u32s(&self.last_hit);
        h.bytes(&self.upg_done);
        h.u32s(&self.inv_kind);
        h.u32s(&self.inv_count);
        h.u32s(&self.stash_kind);
        h.u32s(&self.stash_count);
        h.i32s(&self.item_cd);
        h.u32s(&self.xp_total);
        h.u32s(&self.xp_curve);
        h.u32s(&self.unit_kills);
        h.u32s(&self.unit_deaths);
        h.i32s(&self.aggro.target);
        h.u32s(&self.aggro.until);
        h.u32s(&self.aggro.creep_seen);
        h.u32s(&self.aggro.glyph_until);
        h.u32s(&self.aggro.glyph_ready);
        h.u32s(&self.aggro.glyph_need);
        h.u32s(&self.aggro.glyph_skip);
        h.u32s(&self.wards.expires);
        h.bytes(&self.couriers.phase);
        h.i32s(&self.couriers.target);
        h.i32s(&self.couriers.reach);
        h.i32s(&self.couriers.home_x);
        h.i32s(&self.couriers.home_y);
        h.bytes(&self.camps.is_camp);
        h.i32s(&self.camps.home_x);
        h.i32s(&self.camps.home_y);
        h.bytes(&self.camps.waiting);
        h.u32s(&self.camps.respawn_at);
        h.bytes(&self.grid.flags);
        h.bytes(&self.grid.tiles);
        h.u64s(&[self.doodads.len() as u64]);
        for d in &self.doodads {
            h.u32s(&[d.id]);
            h.bytes(&d.kind.to_le_bytes());
            h.i32s(&[d.x, d.y, d.rotation]);
        }
        h.bytes(&self.grid.heights);
        h.i32s(&[self.grid.height_step]);
        h.i32s(&self.pos_x);
        h.i32s(&self.pos_y);
        h.i32s(&self.vel_x);
        h.i32s(&self.vel_y);
        h.i32s(&self.tgt_x);
        h.i32s(&self.tgt_y);
        h.i32s(&self.hp);
        h.i32s(&self.hp_max);
        h.i32s(&self.radius);
        h.i32s(&self.speed);
        h.i32s(&self.damage);
        h.i32s(&self.armor);
        h.i32s(&self.range_q);
        h.i32s(&self.range_sq);
        h.i32s(&self.period);
        h.i32s(&self.cooldown);
        h.i32s(&self.shield);
        h.i32s(&self.shield_max);
        h.i32s(&self.sight_cells);
        h.bytes(&self.player);
        h.bytes(&self.alive);
        h.i32s(&self.type_of);
        h.u32s(&self.filter);
        h.bytes(&self.response);
        h.i32s(&self.product);
        h.bytes(&self.flying);
        h.bytes(&self.targets);
        h.bytes(&self.kind);
        h.bytes(&self.blocked);
        h.bytes(&self.routed);
        h.bytes(&self.order_kind);
        h.i32s(&self.patrol_x);
        h.i32s(&self.patrol_y);
        h.i32s(&self.amount);
        h.bytes(&self.wstate);
        h.i32s(&self.wtarget);
        h.i32s(&self.wtimer);
        h.i32s(&self.carry);
        h.bytes(&self.carry_kind);
        h.i32s(&self.harvest_amt);
        h.i32s(&self.trainee);
        h.i32s(&self.train_left);
        h.bytes(&self.train_hold);
        h.i32s(&self.resource_a);
        h.i32s(&self.resource_b);
        h.i32s(&self.abil_idx);
        h.i32s(&self.abil_cd);
        h.bytes(&self.abil_rank);
        h.i32s(&self.energy);
        h.u32s(&self.changed_at);
        h.bytes(&self.trig_hit);
        h.bytes(&self.trig_spawn);
        h.bytes(&self.trig_idle);
        h.bytes(&self.trig_death);
        h.bytes(&self.trig_ready);
        h.i32s(std::slice::from_ref(&self.winner));
        h.i32s(&self.buff_kind);
        h.i32s(&self.buff_left);
        h.bytes(&self.healer_player);
        h.i32s(&self.healer_left);
        h.i32s(&self.healer_timer);
        h.i32s(&self.healer_period);
        h.i32s(&self.healer_amt);
        h.i32s(&self.healer_rad);
        h.i32s(&self.healer_x);
        h.i32s(&self.healer_y);
        h.i32s(&self.healer_target);
        h.bytes(&self.m_alive);
        h.i32s(&self.m_timer);
        h.i32s(&self.m_caster);
        h.i32s(&self.m_ability);
        h.i32s(&self.m_target);
        h.i32s(&self.m_x);
        h.i32s(&self.m_y);
        h.bytes(&self.vis);
        h.bytes(&self.researched);
        h.i32s(&self.research_left);
        h.i32s(&self.research_rec);
        h.i32s(&self.loaded_into);
        h.i32s(&self.channel_left);
        h.i32s(&self.channel_ai);
        h.i32s(&self.channel_x);
        h.i32s(&self.channel_y);
        h.i32s(&self.channel_target);
        h.bytes(&self.burrowed);
        h.i32s(&self.burrow_left);
        h.i32s(&self.status_left);
        h.i32s(&self.slow_mul);
        h.bytes(&self.seen);
        h.i32s(&self.seen_x);
        h.i32s(&self.seen_y);
        h.bytes(&self.has_seer.map(|b| b as u8));
        h.bytes(&self.team);
        for r in &self.reveals {
            h.bytes(&[r.p, r.live as u8]);
            h.i32s(&[r.x0, r.y0, r.x1, r.y1, r.until]);
        }
        for a in self.alliance {
            h.bytes(&a.to_le_bytes());
        }
        for v in self.vision {
            h.bytes(&v.to_le_bytes());
        }
        h.bytes(&self.controller);
        h.u64s(&[
            self.stats.deaths as u64,
            self.stats.vision_refreshes as u64,
            self.stats.harvested as u64,
            self.stats.trained as u64,
            self.stats.trains_refused as u64,
            self.stats.casts as u64,
            self.stats.casts_refused as u64,
            self.stats.missiles_lost as u64,
            self.stats.ability_damage as u32 as u64,
            self.stats.retargets as u64,
            self.stats.retargets_deferred as u64,
            self.stats.buffs_dropped as u64,
            self.stats.summoned as u64,
            self.stats.reveals_dropped as u64,
        ]);
        for c in self.stats.refused_by {
            h.u64s(&[c as u64]);
        }
        h.bytes(&[self.stats.last_refusal as u8, self.stats.last_train_refusal as u8]);
        h.0
    }
}
