# 6. Clout
Clout is the signature resource. A hero earns Clout only while an enemy hero can see them. It is the reason to stay in the open, and the reason to be ratioed when you are.

### Earning Clout

| Action | Clout | Condition |
| --- | --- | --- |
| Last hit | +1 | Inside enemy hero vision |
| Deny | +0.5 | Taken from the nearest enemy hero, while that hero sees the creep |
| Hero kill | +10 | None |
| Assist | +4 | None |
| Tower damage | +1 per 100 damage | While an enemy hero sees the tower |
| Enemy ward | 0 | Reveals the warded area, earns nothing |
| Any action unseen | 0 | Gold and XP still earned |

### Clout rules

| Rule | Value |
| --- | --- |
| Reach radius | 300 + 3 × Clout, capped at 600 |
| Stan | Allied creeps inside Reach: +20% damage, +2 armor |
| Go Viral threshold | 100 Clout |
| Go Viral duration | 20 s |
| Go Viral effects | Reach +200; lane spawns +1 melee creep per wave; caster revealed on the global minimap |
| Clout after Go Viral | 25 |
| Ratio on death | Victim loses 50% of Clout, split among killer and assists |
| Decay | Above 50 Clout: −1 every 10 s while unseen for those 10 s |
| Cap on Ratio | At most 200 Clout moves in one death |

### Clout states

Each hero is in one of four states at any time. The state sets what the rest of the map can see.

1. **Quiet.** Below 50 Clout, no enemy vision. Earns gold and XP only.
2. **LIVE.** An enemy hero has the hero in vision. The violet LIVE dot shows on the portrait. Clout is earned.
3. **Viral.** At 100 Clout. Go Viral is active for 20 s. The hero is revealed on the global minimap.
4. **Ratioed.** Death. Clout transfers to the killer and assists. The hero returns to Quiet at spawn.

Rebrand resets a hero to Quiet at 0 Clout.

### Design rules

- Clout is never a stat. It never adds damage or armor directly. Its only effects are Reach, Go Viral, Stan and the Algorithm mood.
- Clout is never shared between teams. The ally tether (section 2) shares a fraction of an ally's Clout, and only with that ally.
- Clout is shown as one number on the top bar, abbreviated (1.2K) and exact on hover.

### Algorithm mood

Each team has one mood, set by the gap between the two teams' total Clout. Mood changes Reach and minimap reveal only. It is not a stat and never adds damage or armor. Numbers are UNVERIFIED.

| Mood | Entered when | Effect |
| --- | --- | --- |
| Front Runner | Team total ahead by 300 Clout or more for 30 s | Its heroes get Reach +50. Its highest-Clout hero shows on the minimap as a Front Runner icon for 20 s every 60 s. |
| Underdog | Team total behind by 300 Clout or more for 30 s | Its heroes get Reach +50. It captures Breaking News zones at double speed (section 12). |
| Even | Neither of the above | No effect. |

A mood ends when the gap falls below 150 Clout for 30 s, so moods do not flicker.
