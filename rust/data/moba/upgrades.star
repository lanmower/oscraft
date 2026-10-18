# MOBA upgrades: tower plating on existing towers, a sharper hero strike for new heroes.
upgrade("tower_plating", "unit", "tower", effect("shield_bonus", p0=20), apply="existing", stacking="add", minerals_per_level=100)
upgrade("sharper_strike", "ability", "hero_strike", effect("damage", p0=2), level=2, apply="new_only", stacking="refuse", minerals_per_level=150)
