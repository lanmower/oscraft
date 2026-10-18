# MOBA research and damage factors.
research("tower_range", "tower", 600, 100, 0, effect("damage", p0=3), applies_to=["ground"])
prerequisite("unit", "objective", "research", "tower_range")
prerequisite("research", "tower_range", "unit", "tower")
damage_factor("explosive", "large", 49152)
damage_factor("concussive", "large", 40960)
