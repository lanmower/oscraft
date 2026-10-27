# 5. Economy
There are three currencies: gold (spent), XP (levels), and Clout (section 6, a Brainrot original). Gold is split into reliable gold (kills, towers, The Trend, bounties) and unreliable gold (creep income). Dying loses only unreliable gold.

### Gold

| Source | Value | Notes |
| --- | --- | --- |
| Starting gold | 600 | Fixed value |
| Passive income | 1 gold per second | Gold each second alive |
| Lane creep last hit | Melee 38-48, ranged 43-53, siege 66-80 | Unreliable. Wave composition in section 3 (UNVERIFIED) |
| Hero kill | 110 + streak bounty + level × 9.9 | Reliable |
| Assist, AoE share | 1 hero: 150 + 6L · 2: 100 + 5L · 3: 40 + 4L · 4: 25 + 3L · 5: 20 + 3L | L is the victim's level. Shared across heroes in range. |
| Streak bounty | 60 up to 480 at a long streak | Ends on death |
| Tower | Team gold per tier (about 160, 200, 240, 280) | Denied tower = 50%. |
| Lurker Cam kill | 100 gold, 25 XP | Enemy ward, section 13 |

### Death and respawn

- **Death cost:** 30 × hero level, taken from unreliable gold.
- **Respawn:** 5 + 3.8 × level seconds (8.8 s at level 1, 100 s at level 25).
- **Ratio on death:** the victim loses 50% of Clout (section 6), split among killers and assists.

### Rebrand

- **Cost:** 100 + 1.5 × level² + 0.25 × game seconds. Spends reliable gold first.
- **Effect:** a Rebrand resets Clout to 0 and ends the respawn timer early.
- **Penalty:** no unreliable gold until the normal respawn timer would have ended. 25% of the skipped timer is added to the next death.
- **Cooldown:** 6 min (360 s).

### Experience

Hero kill XP: 100, 120, 140, 160, 180 for levels 1 to 5, then 100 more per level. Shared among heroes in range. Denied lane creeps give 50% of their XP to the enemy team. Neutral-denied creeps give 35%.

Total XP to reach each level (cumulative, lookup values):

| Level | Total XP | Level | Total XP |
| --- | --- | --- | --- |
| 2 | 200 | 15 | 11,900 |
| 5 | 1,400 | 20 | 20,900 |
| 10 | 5,400 | 25 | 32,400 |

### Clock

- Creeps spawn at 0:00.
- Neutrals spawn at 0:30 and every minute after.
- Boosts spawn every 2 min.
- The Trend opens at about 10:00.
- Bounty boosts spawn at the river at game start, four in total.
