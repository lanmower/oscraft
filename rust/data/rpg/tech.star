# RPG research and damage factors. A quest alarm is researched at the quest giver.
research("quest_alarm", "quest_giver", 600, 0, 0, effect("heal", p0=5, p1=0), applies_to=["ground"])
prerequisite("research", "quest_alarm", "unit", "quest_giver")
damage_factor("concussive", "small", 40960)
