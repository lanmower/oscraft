# Mod scripting

A mod is a Lua chunk. It keeps the original surface: a top-level `on_tick(state)` runs once per
step, and `on_spawned`, `on_attacked`, `on_died`, `on_enter` and `on_leave` receive events. The
ECS-style surface below is additive. It lives on the `game` table and changes nothing above.

## Step order

Each `advance` runs, in this order:

1. The sim tick.
2. Event delivery (spawned, attacked, died, enter, leave).
3. Every system, in registration order, over every living unit.
4. The queued commands, in the order they were queued.
5. Due timers.
6. `on_tick(state)`.

A command queued by `on_tick` or a timer waits for step 4 of the next advance.

## Components

Systems and queries see only the components they name. Every unit view also carries `id`.

| name       | value                         |
|------------|-------------------------------|
| `position` | `{x = ..., y = ...}`, Q16.16  |
| `owner`    | player index                  |
| `hp`       | hit points                    |

`velocity` is not exposed: the sim has no public accessor for it. An unknown name raises
`unknown_component`.

## Surface

- `game.system(name, {components}, fn)` registers a system. `fn(unit)` runs for each living unit
  each step. A duplicate name raises `duplicate_system`. Registration is per load, so a reload
  replaces the systems.
- `game.each({components}, fn)` runs `fn(unit)` for each living unit now, from `on_tick` or a
  system.
- `game.cmd_spawn(kind, x, y[, owner, lane])` queues a spawn at Q16.16 coordinates for `owner` (default 0) in
  `lane` (default 0). An unknown kind raises `unknown_unit` at queue time.
- `game.cmd_order(unit, x, y)` queues a move order.
- `game.cmd_damage(caster, target, dmg)` queues damage from the caster's player.
- `game.cmd_buff(unit, kind, ticks)` queues a buff.
- `game.cmd_cast(unit, slot, target, x, y)` queues an ability cast from the unit's slot at Q16.16 point `x, y`. A `target` of `-1` casts at the point; any other target is a live unit. The sim refuses a cast it cannot make (cooldown, energy, range) and the command then does nothing.
- `game.cmd_attack(attacker, target)` queues an attack order. The attacker moves toward the live target and strikes once when it is in weapon reach. The sim never chases, and each unit otherwise attacks the nearest enemy it can see, so a mod that keeps a target engaged queues the attack again each step.

Queue-time errors (`invalid_unit`, `invalid_damage`, `invalid_buff`, `invalid_slot`) are raised by the call. A
command whose unit died before it applied does nothing.

## Example

```lua
game.system("advance", {"position", "owner"}, function(u)
  if u.owner == 0 then
    game.cmd_order(u.id, u.position.x + 65536, u.position.y)
  end
end)

function on_tick(state)
  state.mine = 0
  game.each({"owner"}, function(u)
    if u.owner == 0 then state.mine = state.mine + 1 end
  end)
end
```

The system queues one move per player-0 unit. The commands apply after every system has run, in
the order they were queued, so the result does not depend on how the systems interleave.

## Checking a mod offline

`mod_check [--fog on|off] <scenario-dir> <mod.lua> [ticks]` builds the scenario image, loads the mod, runs the
ticks (default 100) and prints one `mod-check` line. It exits 0 when the mod loads and every tick
runs. Fog of war is on unless `--fog off`; with it on, a command aimed at a unit the sides cannot see is refused. A load error prints `mod-check load-error` with the file and line; a tick error prints
`mod-check tick-error tick=<n>`. An ok line ends with `refusals=<reason>:<count>,...`, the refusals the sim counted, most frequent first (`none` when there were none). Build it with `cargo build -p mod-host --bin mod_check`.
