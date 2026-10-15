//! FNV-1a over the struct-of-arrays bytes, in a fixed field order. Two runs that
//! agree on this agree on every unit, resource, buff, projectile, fog plane and counter.

use crate::arrays::Sim;

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

impl Sim {
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv(0xcbf2_9ce4_8422_2325);
        h.u64s(&[self.n as u64, self.head as u64, self.ticks as u64, self.rng, self.scan_left as u64]);
        h.bytes(&self.grid.flags);
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
        h.i32s(&self.amount);
        h.bytes(&self.wstate);
        h.i32s(&self.wtarget);
        h.i32s(&self.wtimer);
        h.i32s(&self.carry);
        h.bytes(&self.carry_kind);
        h.i32s(&self.harvest_amt);
        h.i32s(&self.trainee);
        h.i32s(&self.train_left);
        h.i32s(&self.minerals);
        h.i32s(&self.vespene);
        h.i32s(&self.abil_idx);
        h.i32s(&self.abil_cd);
        h.i32s(&self.energy);
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
        h.bytes(&self.m_alive);
        h.i32s(&self.m_timer);
        h.i32s(&self.m_caster);
        h.i32s(&self.m_ability);
        h.i32s(&self.m_target);
        h.i32s(&self.m_x);
        h.i32s(&self.m_y);
        h.bytes(&self.vis);
        h.bytes(&self.has_seer.map(|b| b as u8));
        h.u64s(&[
            self.stats.deaths as u64,
            self.stats.vision_refreshes as u64,
            self.stats.harvested as u64,
            self.stats.trained as u64,
            self.stats.casts as u64,
            self.stats.casts_refused as u64,
            self.stats.missiles_lost as u64,
            self.stats.ability_damage as u32 as u64,
        ]);
        h.0
    }
}
