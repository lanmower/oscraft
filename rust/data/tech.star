# Research, prerequisites and damage factors.
# research(name, building, time, minerals, gas, effect_handle, applies_to=) runs at one building.
research("concussive_rounds", "warden", 1200, 100, 100, effect("damage", p0=3), applies_to=["ground"])
# prerequisite(target_kind, target, requires_kind, requires): kinds are "unit" or "research".
prerequisite("unit", "warden", "unit", "skirmisher")
prerequisite("research", "concussive_rounds", "unit", "warden")
# damage_factor(damage_type, size, factor_q16): 65536 is 1.0.
damage_factor("explosive", "large", 49152)
damage_factor("concussive", "small", 40960)
