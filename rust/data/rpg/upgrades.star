# RPG xp curve: strike gains damage at level 5; cost per level scales linearly with the level.
upgrade("xp_curve_strike", "ability", "strike", effect("damage", p0=1), level=5, apply="existing", stacking="add", resource_a_per_level=100)
