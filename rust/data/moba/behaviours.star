# MOBA behaviours: creeps answer attacks, heroes muster on spawn (respawn), the hero heals when its aura is ready.
behaviour("creep_answer_attack", trigger="attacked", action="attack_nearest", param=0)
behaviour("muster_on_spawn", trigger="spawned", action="flee", param=2000)
behaviour("hero_recast_aura", trigger="ability_ready", action="use_ability", ability="heal_aura")
