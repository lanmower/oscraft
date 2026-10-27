use crate::Sim;

pub fn carry_from(old: &Sim, new: &mut Sim) -> (usize, usize) {
    new.ticks = old.ticks;
    new.adopt_obstacles(old);
    let kinds = new.cat.items.len() as u32;
    new.set_ground(old.ground_items().iter().copied().filter(|g| g.kind <= kinds).collect(), old.ground_next);
    let mut carried = 0;
    for i in 0..new.count() {
        if !new.alive(i) {
            continue;
        }
        let same = i < old.count()
            && old.alive(i)
            && old.player(i) == new.player(i)
            && old.kind_name(i).is_some()
            && old.kind_name(i) == new.kind_name(i);
        if same {
            let (x, y) = old.position_fixed(i);
            new.set_position(i, x, y);
            new.set_health(i, old.hp(i), old.hp_max(i));
            carried += 1;
        } else {
            new.remove_unit(i);
        }
    }
    (carried, old.alive_count().saturating_sub(carried))
}
