# Ability definitions. effect() returns a handle that ability() consumes.
# Ability indices follow declaration order: pulse_shot 0, mend_field 1, bulwark 2, rally 3, purge 4, recall_pulse 5.
ability("pulse_shot", 24, 0, "unit", [effect("damage", p0=6)], target_filters=["ground"], projectile_speed_milli=12000)
ability("mend_field", 96, 50, "point", [effect("heal", p0=12, p1=2000), effect("buff", p0=1, p1=3)])
ability("bulwark", 180, 75, "none", [effect("shield_bonus", p0=30), effect("vital_fraction", p0=1, p1=32768, p2=1, p3=1)])
ability("rally", 120, 25, "point", [effect("issue_order", p0=1, p1=1), effect("create_healer", p0=4, p1=3000, p2=24, p3=240)])
ability("purge", 150, 40, "unit", [effect("remove_behaviour", p0=1, p1=2000)], target_filters=["biological"])
ability("recall_pulse", 300, 100, "none", [effect("use_calldown", p0=0)])
