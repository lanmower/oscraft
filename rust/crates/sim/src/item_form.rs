use crate::arrays::Sim;
use crate::inventory::{item_kind, INV_SLOTS};
use crate::loader::Effect;

pub(crate) const K_ITEM_FORM: u16 = 36;
pub(crate) const FORM_SWAP: i32 = 0;
pub(crate) const FORM_SPEND_CHARGE: i32 = 1;

pub(crate) fn params_valid(p1: i32, p2: i32, p3: i32) -> bool {
    (FORM_SWAP..=FORM_SPEND_CHARGE).contains(&p1) && p2 >= 0 && p3 >= 0
}

impl Sim {
    pub(crate) fn item_form_effect(&mut self, caster: usize, eff: Effect) {
        let using = self.expiry.casting_slot;
        if using < 0 || using as usize >= INV_SLOTS {
            return;
        }
        let key = caster * INV_SLOTS + using as usize;
        let Some(form) = usize::try_from(eff.p0).ok().filter(|&f| f < self.cat.items.len()) else {
            return;
        };
        if self.inv_count[key] == 0 {
            return;
        }
        if eff.p1 == FORM_SPEND_CHARGE && self.inv_count[key] > 1 {
            self.change_items(caster, |sim| sim.inv_count[key] -= 1);
            return;
        }
        let count = if eff.p2 > 0 { eff.p2 as u32 } else { u32::from(self.cat.items[form].charges).max(1) };
        let lock = self.ticks.saturating_add(eff.p3 as u32);
        if self.replace_item(caster, using as usize, item_kind(form), count).is_err() {
            return;
        }
        if eff.p3 > 0 {
            self.item_lock_until[key] = lock;
            self.item_lock_kind[key] = item_kind(form);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::arrays::Sim;
    use crate::inventory::{item_kind, INV_SLOTS};
    use crate::{fixed, load};

    fn pack_sim() -> Sim {
        let image = osdt_data::generate(&crate::test_pack_dir()).expect("pack image");
        let mut s = load(&image, 1024).expect("pack sim");
        s.set_fog(false);
        s
    }

    fn hero(s: &mut Sim, player: u8, name: &str) -> usize {
        let kind = s.catalog().unit_index(name).expect("hero kind") as u32;
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        let unit = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find_map(|(x, y)| s.spawn_for(player, kind, fixed::from_int(x), fixed::from_int(y)))
            .expect("a free cell");
        let (x, y) = s.position_fixed(unit);
        s.attribute_unit(unit, x, y);
        unit
    }

    fn give(s: &mut Sim, unit: usize, name: &str, charges: u32) -> usize {
        let item = s.catalog().item_index(name).unwrap_or_else(|| panic!("pack item {name}"));
        assert!(s.inventory_add(unit, item_kind(item), charges), "{name} fits");
        (0..INV_SLOTS).find(|&k| s.inventory_slot(unit, k).0 == item_kind(item)).expect("slot")
    }

    fn held(s: &Sim, unit: usize, slot: usize) -> String {
        let (kind, _) = s.inventory_slot(unit, slot);
        s.catalog().name(s.catalog().items[kind as usize - 1].name).to_owned()
    }

    fn ready(s: &mut Sim, unit: usize, slot: usize) {
        s.set_item_cooldown(unit, slot, 0);
        s.energy[unit] = 1000;
    }

    #[test]
    fn power_treads_use_cycles_the_attribute_and_keeps_the_slot() {
        let mut s = pack_sim();
        let h = hero(&mut s, 0, "antimage");
        let slot = give(&mut s, h, "power_treads", 1);
        assert_eq!((s.item_bonus(h, "agility"), s.item_bonus(h, "strength")), (9, 0));
        for want in ["power_treads_strength", "power_treads_intelligence", "power_treads"] {
            ready(&mut s, h, slot);
            assert!(s.use_item(h, slot, -1, 0, 0), "the toggle casts");
            assert_eq!(held(&s, h, slot), want);
        }
        assert_eq!((s.item_bonus(h, "agility"), s.item_bonus(h, "attack_speed")), (9, 25));
    }

    #[test]
    fn black_king_bar_uses_walk_the_form_chain_and_the_last_form_stays() {
        let mut s = pack_sim();
        let h = hero(&mut s, 0, "antimage");
        let slot = give(&mut s, h, "black_king_bar", 1);
        for want in ["black_king_bar_9", "black_king_bar_8", "black_king_bar_7", "black_king_bar_6", "black_king_bar_5", "black_king_bar_5"] {
            ready(&mut s, h, slot);
            assert!(s.use_item(h, slot, -1, 0, 0));
            assert_eq!(held(&s, h, slot), want);
        }
        assert!(s.status_left(h, crate::status::SPELL_IMMUNE) > 0, "the avatar is up");
    }

    #[test]
    fn diffusal_spends_a_charge_a_use_and_becomes_the_empty_copy_at_none() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        let slot = give(&mut s, a, "diffusal_blade", 8);
        for left in (1..8).rev() {
            ready(&mut s, a, slot);
            assert!(s.use_item(a, slot, b as i32, 0, 0), "purge casts");
            assert_eq!((held(&s, a, slot), s.inventory_slot(a, slot).1), ("diffusal_blade".to_owned(), left));
        }
        ready(&mut s, a, slot);
        assert!(s.use_item(a, slot, b as i32, 0, 0));
        assert_eq!((held(&s, a, slot), s.inventory_slot(a, slot).1), ("diffusal_blade_empty".to_owned(), 1));
        assert_eq!(s.item_bonus(a, "agility"), 20, "the empty copy keeps its stats");
    }

    #[test]
    fn spell_block_absorbs_one_unit_cast_without_its_effect_and_logs_it() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        let slot = give(&mut s, a, "diffusal_blade", 8);
        give(&mut s, b, "sphere", 1);
        let energy = s.energy[a];
        ready(&mut s, a, slot);
        assert!(s.use_item(a, slot, b as i32, 0, 0), "the cast is made");
        assert_eq!(s.take_blocks().len(), 1, "the block is logged");
        assert_eq!(s.inventory_slot(a, slot).1, 8, "no effect ran, so no charge was spent");
        assert!(s.energy[a] < 1000 || energy < 1000, "the cost was paid");
        assert!(s.take_casts().is_empty(), "an absorbed cast does not reach on_cast");
    }

    #[test]
    fn true_strike_ignores_evasion() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        s.smod_add(b, crate::spell::S_EVASION, 100);
        let full = s.hp(b);
        s.strike_pct(a, b, 100);
        assert_eq!(s.hp(b), full, "a victim with 100 percent evasion dodges");
        give(&mut s, a, "monkey_king_bar", 1);
        s.strike_pct(a, b, 100);
        assert!(s.hp(b) < full, "true strike lands");
    }

    #[test]
    fn hero_attackers_meet_the_full_block_of_a_poor_mans_shield() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        let before = s.hp(b);
        s.strike_pct(a, b, 100);
        let plain = before - s.hp(b);
        s.set_health(b, before, before);
        give(&mut s, b, "poor_mans_shield", 1);
        s.strike_pct(a, b, 100);
        assert_eq!(before - s.hp(b), (plain - fixed::from_int(16)).max(0), "a hero attacker meets the 100 percent block of 16");
    }

    #[test]
    fn quelling_blade_adds_damage_against_non_heroes_only() {
        let mut s = pack_sim();
        let a = hero(&mut s, 0, "antimage");
        let enemy = hero(&mut s, 1, "antimage");
        let kind = s.catalog().unit_index("creep").expect("creep kind") as u32;
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        let creep = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .find_map(|(x, y)| s.spawn_for(1, kind, fixed::from_int(x), fixed::from_int(y)))
            .expect("a free cell");
        let mut dealt = |s: &mut Sim, victim: usize| {
            let max = s.hp_max(victim);
            s.set_health(victim, max, max);
            s.strike_pct(a, victim, 100);
            max - s.hp(victim)
        };
        let (creep_bare, hero_bare) = (dealt(&mut s, creep), dealt(&mut s, enemy));
        give(&mut s, a, "quelling_blade", 1);
        let (creep_armed, hero_armed) = (dealt(&mut s, creep), dealt(&mut s, enemy));
        assert!(creep_armed > creep_bare, "32 percent more damage to a creep ({creep_bare} then {creep_armed})");
        assert_eq!(hero_armed, hero_bare, "a hero victim takes no bonus");
    }

    #[test]
    fn a_black_king_bar_use_locks_the_new_form_against_dropping() {
        let mut s = pack_sim();
        let h = hero(&mut s, 0, "antimage");
        let slot = give(&mut s, h, "black_king_bar", 1);
        assert_eq!(s.item_lock_left(h, slot), 0);
        ready(&mut s, h, slot);
        assert!(s.use_item(h, slot, -1, 0, 0));
        assert_eq!(s.item_lock_left(h, slot), 1200, "75 seconds after the first use");
        assert!(s.drop_item_to_ground(h, slot, 0, 0).is_err(), "a locked form is not dropped");
        for _ in 0..1200 {
            s.tick();
        }
        assert_eq!(s.item_lock_left(h, slot), 0);
        assert!(s.drop_item_to_ground(h, slot, 0, 0).is_ok(), "the lock lapses");
    }

    #[test]
    fn a_borrowed_spell_block_absorbs_one_cast_and_ends() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        let slot = give(&mut s, a, "diffusal_blade", 8);
        s.set_spell_block(b, 256);
        assert_eq!(s.spell_block_left(b), 256);
        ready(&mut s, a, slot);
        assert!(s.use_item(a, slot, b as i32, 0, 0));
        assert_eq!(s.take_blocks().len(), 1);
        assert_eq!(s.spell_block_left(b), 0, "the borrowed block is spent");
        assert_eq!(s.inventory_slot(a, slot).1, 8, "no charge was spent on the absorbed cast");
        ready(&mut s, a, slot);
        assert!(s.use_item(a, slot, b as i32, 0, 0));
        assert!(s.take_blocks().is_empty(), "the second cast lands");
    }

    #[test]
    fn a_true_sight_carrier_reveals_a_cloaked_unit() {
        let mut s = pack_sim();
        let (a, b) = (hero(&mut s, 0, "antimage"), hero(&mut s, 1, "antimage"));
        let kind = s.catalog().unit_index("techies_land_mine").expect("mine kind") as u32;
        let (x, y) = s.position_fixed(b);
        let mine = s.spawn_for(1, kind, x + fixed::from_int(1), y).expect("mine");
        assert!(!s.detected(0, mine), "a cloaked mine is hidden from an unequipped team");
        give(&mut s, a, "gem", 1);
        assert!(s.detected(0, mine), "the gem carrier reveals it");
    }

    #[test]
    fn counters_survive_a_save_and_load() {
        let mut s = pack_sim();
        let h = hero(&mut s, 0, "antimage");
        s.set_counter(h, "bkb_store", 7);
        let bytes = s.save_state();
        s.set_counter(h, "bkb_store", 0);
        s.load_state(&bytes).expect("state loads");
        assert_eq!(s.counter(h, "bkb_store"), 7);
    }
}
