# RPG quest hooks: wolves respond when attacked (clear-the-den quest), the quest giver mends when its ability is ready.
behaviour("wolf_pack_response", trigger="attacked", action="attack_nearest", param=0)
behaviour("quest_giver_mend", trigger="ability_ready", action="use_ability", ability="mend")
