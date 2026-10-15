# Upgrade definitions. upgrade(name, kind, target, effect_handle, level=, apply=, stacking=)
# kind: "unit" or "ability". apply: "new_only" or "existing". stacking: "replace", "add" or "refuse".
upgrade("dense_plating", "unit", "warden", effect("shield_bonus", p0=20), apply="existing", stacking="add")
upgrade("sharper_pulse", "ability", "pulse_shot", effect("damage", p0=2), level=2, apply="new_only", stacking="refuse")
