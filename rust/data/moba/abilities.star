# MOBA ability definitions. effect() returns a handle that ability() consumes.
# Ability indices follow declaration order: hero_strike 0, tower_shot 1, heal_aura 2, dash 3.
ability("hero_strike", 10, 0, "unit", [effect("damage", p0=14)], target_filters=["ground"])
ability("tower_shot", 12, 0, "unit", [effect("damage", p0=20)], target_filters=["ground"], projectile_speed_milli=10000)
ability("heal_aura", 40, 30, "none", [effect("heal", p0=10, p1=1500), effect("buff", p0=1, p1=3)])
ability("dash", 60, 0, "point", [effect("move", p0=6000)])
# heal_aura is channelled: the caster is held for 24 ticks.
channel("heal_aura", 24)
