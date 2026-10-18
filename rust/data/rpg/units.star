# RPG units: the adventurer carries inventory, the quest giver is a neutral non-combatant, wolves are the quest targets.
unit("adventurer", 300, 2800, 400, build_time=0, capacity=0, abilities=["strike", "mend"], filters=["ground", "light", "biological", "army"], weapon_damage=8, weapon_range_milli=1200, weapon_period=20, sight_milli=9000, resource="none")
unit("wolf", 120, 3000, 350, build_time=0, capacity=0, abilities=["bite"], filters=["ground", "light", "biological"], weapon_damage=5, weapon_range_milli=1000, weapon_period=16, sight_milli=7000)
unit("quest_giver", 1000, 0, 400, build_time=0, capacity=0, abilities=["mend"], filters=["ground", "biological"], sight_milli=5000)
