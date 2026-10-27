use crate::arrays::Sim;
use crate::fixed;

impl Sim {
    pub fn morph(&mut self, i: usize, kind: u32) -> bool {
        if i >= self.n || self.alive[i] == 0 || kind as usize >= self.cat.units.len() || self.type_of[i] < 0 {
            return false;
        }
        let def = self.cat.units[kind as usize];
        let (hp, hp_max) = (self.hp[i] as i64, self.hp_max[i].max(1) as i64);
        self.type_of[i] = kind as i32;
        self.filter[i] = def.filter;
        self.response[i] = def.response;
        self.flying[i] = u8::from(def.is_air());
        self.kind[i] = self.classify(def);
        self.radius[i] = fixed::from_milli(def.radius);
        self.speed[i] = fixed::from_milli(def.speed);
        self.armor[i] = fixed::from_int(def.armor);
        if def.has_weapon() {
            self.damage[i] = fixed::from_int(def.weapon_damage);
            self.range_q[i] = fixed::from_milli(def.weapon_range);
            self.range_sq[i] = fixed::mul(self.range_q[i], self.range_q[i]);
            self.period[i] = def.weapon_period.max(1);
        }
        self.sight_cells[i] = fixed::round(fixed::from_milli(def.sight)).max(0);
        let new_max = fixed::from_int(def.health + self.stat(i, crate::spell::S_HEALTH));
        self.hp_max[i] = new_max;
        self.hp[i] = ((new_max as i64 * hp / hp_max) as i32).clamp(1, new_max.max(1));
        if self.is_attributed(i) {
            let kept = self.hp[i] as i64 * 1000 / new_max.max(1) as i64;
            self.apply_attr_stats(i);
            let max = self.hp_max[i];
            self.hp[i] = ((max as i64 * kept / 1000) as i32).clamp(1, max.max(1));
        }
        self.touch(i);
        true
    }

    pub fn set_flying(&mut self, i: usize, on: bool) {
        if i < self.n && self.alive[i] != 0 {
            self.flying[i] = u8::from(on);
            self.touch(i);
        }
    }
}
