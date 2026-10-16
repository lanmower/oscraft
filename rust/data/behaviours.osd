# Behaviour definitions. behaviour(name, trigger, action, param=, ability=)
# Triggers: idle, attacked, spawned, ability_ready (ability_ready requires ability=).
behaviour("guard_patrol", trigger="idle", action="use_ability", param=0, ability="pulse_shot")
behaviour("answer_attack", trigger="attacked", action="attack_nearest", param=0)
behaviour("muster_on_spawn", trigger="spawned", action="flee", param=2000)
behaviour("recast_mend", trigger="ability_ready", action="use_ability", ability="mend_field")
