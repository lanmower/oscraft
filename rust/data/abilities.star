# Ability definitions. effect() returns a handle that ability() consumes.
ability("pulse_shot", 24, 0, "unit", [effect("damage", p0=6)], filter=["ground"], projectile_speed_milli=12000)
ability("mend_field", 96, 50, "point", [effect("heal", p0=12), effect("buff", p0=1, p1=3)])
