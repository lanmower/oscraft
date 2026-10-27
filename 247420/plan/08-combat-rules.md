# 8. Combat rules
### Damage types

| Type | Reduced by | Blocked by |
| --- | --- | --- |
| Physical | Armor | Evasion, miss chance |
| Magical | Magic resistance | Spell immunity |
| Pure | Nothing | Nothing |
| HP removal | Nothing | Nothing; HP removal ignores spell immunity |

### Armor and magic resistance

- Physical damage multiplier: 1 − (0.06 × A) / (1 + 0.06 × |A|), where A is armor. Negative armor amplifies damage.
- Magic resistance: heroes start at 25%. Each source multiplies. Two sources at 25% each give 1 − 0.75 × 0.75 = 43.75%.

### Attack speed

- Attacks per second = (100 + bonus attack speed) / (BAT × 100).
- Total attack speed is capped at 600.
- Attack animation: attack point, then backswing. Animation canceling is allowed.

### Evasion and miss

- Evasion uses pseudo-random distribution, checked on impact.
- Miss chance (for example, uphill attacks) is separate from evasion.
- True strike ignores evasion. Sources: specific items and the Hot Take archetype.

### Lifesteal and amplification

- Attack lifesteal: items and a few hero abilities.
- Spell lifesteal: none in v1.
- Spell amplification: Intelligence (section 7) plus one item.
- Cooldown reduction: Refresh Page (cooldown reset) and the Unboxing modifier only.

### Disables

| Disable | Effect | Break rule |
| --- | --- | --- |
| Stun | No actions or movement | Stun cannot be broken |
| Polymorph | Transforms the target | Dispelled by strong dispel |
| Silence | No abilities, items still work | Strong dispel removes it |
| Root | No movement, abilities work | Dispelled by basic dispel |
| Sleep | Stun that breaks on damage | Breaks on any damage |
| Taunt | Forced to attack the source | Lasts its duration |
| Disarm | No attacks, abilities work | Dispelled by basic dispel |
| Fear | Forced to move away | Lasts its duration |
| Cyclone | Airborne, no actions | Stun-equivalent, dispelled by strong dispel |

Rules:

- Hard disable cap: 3 hard disables per team in any 10 s window. A fourth is shortened to 0.5 s.
- Purge: basic dispel removes most buffs and root. Strong dispel removes polymorph, silence and cyclone.
- Spell immunity blocks magical damage and disables, not pure damage or HP removal.

### Repost chain (Brainrot original)

A hard disable or ultimate that lands on an enemy starts a repost chain.

| Property | Value |
| --- | --- |
| Trigger | Hard disable or ultimate lands on an enemy |
| Radius | 600 |
| Stack effect | +8% attack speed or spell amp per stack |
| Spread | Any ally who damages the target passes a stack to allies within 600 of them |
| Max stacks | 5 |
| Decay | 4 s |

Stacks show as dotted arcs between heroes. Repost arcs are capped at five on screen.
