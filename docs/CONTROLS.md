# Controls

This is the control scheme as the code implements it. The keyboard table comes from the `keys` section of `scenarios/moba/moba_ui.json`, the Lua key resolution is `resolve_key` in `scenarios/moba/controls.lua`, and the pad and lock behaviour is in `js/client/twin_stick.js`, `js/client/play.js` and `js/client/hero_view_lock.js`.

## Hero view lock

The lock makes the hero the thing the camera follows and the thing the arrow keys and the left stick move. It is engaged by the `#hero-lock` button in the HUD. There is no key for it.

Engaging the lock sends `one_hand` on to the match. While it is on:

- The left stick and the arrow keys walk the hero. Movement is relative to the camera yaw.
- Casting a targeted ability (Q, W, E, R, the item keys, or a pad button) fires at the best unit in reach. The aim is ignored, so a right-stick aim does not change the target. A unit-target ability with no unit in reach is refused (noTarget). A point-target ability with no unit in reach fires at the hero's own position.
- A walking hero strikes its target in reach, because one-hand sets the strike-on-move flag.

The lock is released by the `#hero-lock` button again, or by any explicit camera move that takes the camera off the hero: a minimap drag, a right-button drag pan, a mod camera drag, a podium or flow camera restore, and the `window.oscraft.centreOn` debug hook. Each release sends `one_hand` off. A minimap centre click does nothing while the lock is on. A targeted cast from the keyboard then arms a target. With quick-cast on for that slot, the arm fires on key release at the pointer's world point, and a unit-target ability with no unit there is refused (noTarget). Otherwise the next click places the target. Quick-cast is toggled per slot by the `quickcast` ui call. A pad cast uses the right stick's aim point.

Without the lock, the arrow keys and the left stick pan the camera.

## Gamepad

The pad is read through the Gamepad API. The layout below is the standard mapping.

| Input | Lock on | Lock off |
| --- | --- | --- |
| Left stick | Walks the hero, forward is away from the camera | Pans the camera |
| Right stick | Sets an aim point; ignored by casts, still used for the key world point (see below) | Sets the aim point for a cast, 8 cells from the hero in the camera direction |
| Buttons 0 to 3 (A, B, X, Y) | Cast slots 0 to 3, the same as Q, W, E, R, at the best unit in reach. The aim is ignored | Cast slots 0 to 3. A targeted cast goes to the right-stick aim point (the hero's own position while the stick is at rest). A no-target ability fires at the hero, whatever the aim |
| Other buttons | Not bound | Not bound |

The sticks have a deadzone of 0.2, applied to each axis. Past it the value is rescaled to 0 to 1. With the lock off that scales the camera pan speed. With the lock on the left stick sets only a direction: the hero is ordered to a point 64 cells ahead in that direction, so deflection does not change its walk speed. The y axis is inverted so pushing a stick up is forward. A button cast fires once per press, not once per frame it is held.

Buttons 0 to 3 cast whether or not the lock is engaged, because the pad is polled every frame. Only the stick roles change with the lock.

## Keyboard

Keys are forwarded to the match, which decides what they do. For each key, the first matching rule wins, in this order.

0. A quick-cast release and Escape are handled first. Then, with a courier selected (not a hero), E sends the courier home when it can, and the courier keys act. A custom binding therefore cannot take a courier letter. Custom keys from the pack's `custom` table (none by default) come next and override the rest of this list.
1. An ability hotkey from the 6.88g ability table (`scenarios/moba/ability_keys.lua`) casts that ability, and Ctrl with it levels the skill.
2. Ctrl plus 1 to 9 sets a control group. Plain 1 focuses the hero. Plain 2 to 9 recalls a group.
3. The keys below.
4. Q, W, E, R cast slots 0 to 3, and Ctrl plus Q, W, E, R levels them.

| Key | Action |
| --- | --- |
| Q W E R | Cast slots 0 to 3 (unless an ability hotkey claims the key) |
| Ctrl + Q W E R | Level skill slots 0 to 3 |
| A | Attack-move command |
| S | Stop |
| H | Hold position |
| Space | Centre the camera on the hero |
| M | Move command |
| P | Patrol |
| Tab | Scoreboard |
| K | Networth |
| F4 | Shop |
| F1 | Select the hero |
| F8 | Select the courier |
| Z X C V B N | Item slots 0 to 5 |
| Num 7 8 4 5 1 2 | Item slots from the item pad, in the same order as the items |
| E | Courier home |
| F | Glyph |
| D A W R F G, with a courier selected | Courier drop, pick up, ping, burst, secret shop, skin. These replace the hero action for those letters while a courier is selected |
| ? | Controls help |
| G, V (hold) | Ping wheel. A release that picks no ping acts as the key normally: G toggles ambient effects and sounds, V fires item slot 3 |
| Shift M | Toggle mute |

Arrow keys: in the lock they walk the hero, without it they pan the camera.

A no-target ability (no unit or point target) has nothing to pick. From the keyboard it fires at the pointer's world point, or at the hero when the key carries none. From the pad it fires at the hero, with the lock on or off.

## Right stick and keyboard together

The right stick's aim point is also the world point that a keyboard key carries to the match while the stick is held. Otherwise the key carries the pointer's world point. This applies with the lock on and off. A key pressed while the right stick is held therefore reaches the match with the stick's point, not the pointer's. This is the behaviour as it stands; it is documented, not changed.

## The unlocked pad

With the lock off the left stick pans the camera, but the pad is not limited to that. The buttons cast, and the right stick sets the aim for those casts.

## Casting, range and refusals

A targeted cast whose target is out of reach walks the hero to 90 percent of the cast range and fires when it arrives. The walk waits up to 10 seconds; after that the cast is refused (`range`). This applies with the lock on and off.

With the lock on, a unit-or-point ability with no unit in reach fires at the hero's own position. A plain unit ability with no unit in reach is refused (`noTarget`). Other refusals: `noHero` when no hero casts, `noAbility` when the slot holds no ability, and `cooldown` while the slot is cooling down.
