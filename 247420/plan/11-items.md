# 11. Items
### Inventory

- Six item slots, plus a six-slot stash. Backpack: three slots, see section 21.
- Consumables stack. Stock-limited items have a restock timer (Lurker Cam, Smoke of Deniability).
- Sell-back: full refund within 10 s of purchase, 50% after that. Screenshot and Gamer Fuel cannot be sold.
- Drop-on-death items: Rapier, Unironic and Gem of Subtext. Their holder drops them on death, visible to all.
- Items that take an active use a shared item cooldown per item, not per hero.

### Shop

- The Spawn Lounge shop is run by The Plug, the shop NPC (section 18). It sells every component, every Tier 1 finished item and the consumable wards.
- The Back Alley Shop sells the Tier 2 and Tier 3 finished items. The two Side Shops sell components only (section 4).
- Purchases spend reliable gold first, then unreliable gold.

### Component tree

Twelve components. Tier 1 and Tier 2 components combine into finished items. Tier 3 and Tier 4 finished items combine finished items with components.

| Component | Tier | Effect |
| --- | --- | --- |
| Cheap Dagger | 1 | Bonus damage |
| Drafted Note | 1 | Bonus mana |
| Sticky Note | 1 | Ward component (vision) |
| Hot Sauce Bottle | 1 | Small heal |
| Ring Light | 1 | Vision radius +50 |
| Envelope | 1 | Delivery Drone speed +10% |
| Old Hoodie | 1 | Bonus armor |
| Rubber Duck | 2 | Attack speed |
| Burner Phone | 2 | Spell amplification |
| Charger | 2 | Mana regeneration |
| Cold Brew | 2 | Health regeneration |
| Tripod | 2 | Attack range |

Finished items by tier (built from the components):

- **Tier 1:** Lurker Cam (Sticky Note + Ring Light), Dust of Receipts (Drafted Note + Sticky Note), Hot Sauce (Hot Sauce Bottle + Drafted Note), Link in Bio Scroll (Envelope + Old Hoodie), Envelope With Legs (Envelope + Sticky Note), Magic Enthusiast (Drafted Note + Hot Sauce Bottle), Spoiler Tag (Sticky Note + Drafted Note).
- **Tier 2:** Ragequit Dagger, Gem of Subtext, Bottle of Copium, Smoke of Deniability, Grindset, Gas Station Sushi, Lo-Fi Beats, Boots of Scrolling, Anon Mask, Confident Answer, Prompt Injection, Sycophant.
- **Tier 3:** Participation Trophy, Heart of Brainrot, Sleep Mode, Reply All, Co-Working Space, Rescheduled, Pro Plan, Dunk, Heavenly Feedback, Ghost Mode, Drain Blade, Lite Plan.
- **Tier 4:** Refresh Page, Rapier Unironic, Dark Mode Shield, Main Character Energy, Sponsored Style, Monetized, Sheep Check.

Cost bands (match the roster costs; the balance pass may move them):

| Tier | Cost band (gold) |
| --- | --- |
| Component 1 | 50 to 100 |
| Component 2 | 200 to 225 |
| Finished tier 1 | 65 to 300 |
| Finished tier 2 | 600 to 1,000 |
| Finished tier 3 | 1,400 to 2,400 |
| Finished tier 4 | 3,000 to 6,000 |

Assembly is discounted. A finished item may cost less than the sum of its components; the listed cost is the one paid, and the balance pass moves bands, not individual costs (section 20, decision 17).

### Item roster

Every item holds the job of a classic item. The internal parity map pairs them; it is not part of the plan or the shipped build. Component and neutral item flavor lines are in plan/23-voice-and-banter.md.

| Brainrot item | Function | Flavor |
| --- | --- | --- |
| Ragequit Dagger | Blink, short disengage | Logs off from your current position. |
| Participation Trophy | One revive at the Spawn Lounge | Everyone gets one. |
| Refresh Page | Resets all cooldowns | Results may vary. |
| Rapier, Unironic | High-value item, drops on death | Dropped on death. Visible to all. |
| Gem of Subtext | Reveals invisible units and wards | Reads between the lines. |
| Bottle of Copium | Restores HP and mana over time | You are fine. Totally fine. For 12 seconds. |
| Heart of Brainrot | Large HP pool. Nearby allies gain 5% less Clout. | Your allies are slightly less sure of themselves. |
| Lurker Cam | Vision. Enemies see the reveal, earn no Clout. | Watching, not posting. |
| Spoiler Tag | True sight ward | Reveals what is hidden. Also what is coming. |
| Dust of Receipts | Reveals invisible units in radius | The proof is in the dust. |
| Smoke of Deniability | Invisibility cloud for allies | Nothing happened here. |
| Gas Station Sushi | Heals, restores trees | Sell-by date is a suggestion. |
| Lo-Fi Beats | Restores mana over time | Beats to farm to. |
| Hot Sauce | Instant heal | Not for beginners. |
| Link in Bio Scroll | Teleports to base | Stay a while. |
| Boots of Scrolling | Movement speed | Infinite scroll, finite boots. |
| Sleep Mode | Toggles attribute and movement | Switch modes. Never fully off. |
| Dark Mode Shield | Brief spell immunity | Works everywhere. |
| Reply All | Reflects damage | Everyone sees what you said. |
| Co-Working Space | Heal aura for the team | Good vibes, shared desk, one outlet. |
| **Anon Mask** | Fade, invisibility on use | Nobody knows who did this. |
| Main Character Energy | Burns nearby enemies | Everyone is looking. You noticed. |
| Sponsored Style | Creates illusions | Duplicates you. Disclosed. |
| Rescheduled | Disables and displaces | Moved to a later time. |
| Pro Plan | Upgrades the ultimate per hero | Upgrade unlocked. |
| Monetized | Converts creep kills to gold | Gold from creeps. Also from attention. |
| Sheep Check | Polymorph on the target | Deliberate. Sheepish. |
| Dunk | Silence and bonus damage | Silenced. Still right. |
| Heavenly Feedback | Disarms and slows | Feedback delivered in person. |
| Magic Enthusiast | Heals and grants magic resistance | Charges from every spell. |
| Ghost Mode | Phases through units while moving | Not here. Not now. |
| Envelope With Legs | Delivers items to base | Late. Always. |
| Confident Answer | Attacks reveal the target's last position for 2 s, 20% chance | Delivered with total certainty and no sources. |
| Prompt Injection | The next ability the target casts hits its nearest ally | Adds one line to the instructions, and the instructions obey. |
| Sycophant | Aura: allies inside gain Reach +100 | Agrees with everyone nearby, which everyone mistakes for friendship. |
| Grindset | Attack speed and damage after a kill | The grind does not stop. Neither do you. |
| Drain Blade | Attack lifesteal after an attack lands | Take their energy. Literally. |
| Lite Plan | Adds one second effect to a basic ability | The budget version of an upgrade. It still counts. |

Design notes:

- Anon Mask replaces Anonymous Blade. It is an invisibility item. It is not a blink, and the name now matches the function.
- The Anonymous Poster ability Anonymous Dagger keeps its name, since it throws a dagger.
- Pro Plan is the one ultimate upgrade per hero. It is the one hero-wide upgrade, and is named for what it does.
- Attack-modifier stacking rules are simplified. Orbs do not stack, which keeps the rule simple.

### Stats and costs

| Item | Cost (gold) | Effect |
| --- | --- | --- |
| Cheap Dagger | 100 | +12 damage |
| Drafted Note | 90 | +25 mana, +0.5 mana regen |
| Sticky Note | 75 | Ward component. Reveal radius +300 |
| Hot Sauce Bottle | 50 | Heal 120 instantly |
| Ring Light | 75 | Vision radius +50 |
| Envelope | 80 | Delivery Drone speed +10% |
| Old Hoodie | 90 | +2 armor |
| Rubber Duck | 225 | +10% attack speed |
| Burner Phone | 200 | +6% spell amplification |
| Charger | 225 | +2 mana regen |
| Cold Brew | 200 | +2 HP regen |
| Tripod | 200 | +75 attack range |
| Lurker Cam | 65 | Vision ward, 6 min, 2 stock, 4 max |
| Dust of Receipts | 80 | Reveals invisible units in 900 radius for 3 s |
| Hot Sauce | 150 | Heal 300 instantly |
| Link in Bio Scroll | 120 | Teleport to base after a 3 s channel |
| Envelope With Legs | 150 | Delivery Drone speed +25% |
| Magic Enthusiast | 300 | Heal 120 on spell cast, +8% magic resistance |
| Spoiler Tag | 75 | True sight ward, true sight 900, 6 min |
| Ragequit Dagger | 900 | Blink 1,100 range, 12 s cooldown, +20 damage |
| Gem of Subtext | 800 | True sight 900 radius, drops on death |
| Bottle of Copium | 800 | Restores 60 HP and mana per second for 12 s |
| Smoke of Deniability | 700 | Group invisibility 6 s, 11 min restock |
| Grindset | 700 | +3 HP regen, +1 mana regen |
| Gas Station Sushi | 750 | Heal 115 over 16 s |
| Lo-Fi Beats | 650 | +1.5 mana regen, +1 armor |
| Boots of Scrolling | 600 | +80 move speed |
| Anon Mask | 800 | Fade for 5 s on use, 90 s cooldown, +20 damage |
| Participation Trophy | 1,600 | Revive at the Spawn Lounge once, 5 min |
| Heart of Brainrot | 2,400 | +600 HP, +5 HP regen. Nearby allies gain 5% less Clout |
| Sleep Mode | 1,500 | Toggle between attribute and movement bonuses |
| Reply All | 1,800 | Reflects 35% of damage taken for 4 s |
| Co-Working Space | 2,000 | Heal aura 150 radius, 4 HP per second |
| Rescheduled | 1,800 | Disable 1.5 s, displace 250, 7 s cooldown |
| Pro Plan | 2,200 | Upgrades the ultimate per hero |
| Dunk | 1,800 | Silence 4 s, +40 damage |
| Heavenly Feedback | 1,800 | Disarm 2 s, slow 20% |
| Ghost Mode | 1,600 | Move through units while moving |
| Refresh Page | 5,000 | Resets all cooldowns, 280 s cooldown |
| Rapier, Unironic | 6,000 | +200 damage. Drops on death, visible to all |
| Dark Mode Shield | 4,500 | Spell immunity 4 s |
| Main Character Energy | 4,200 | Aura burns nearby enemies for 150 per second |
| Sponsored Style | 4,000 | Two illusions for 15 s |
| Monetized | 3,500 | Creep kills give +25% gold |
| Sheep Check | 3,000 | Polymorph 3 s, 15 s cooldown |
| Lite Plan | 1,400 | Adds one second effect to a basic ability (section 21). Standalone; not built from Pro Plan |
| Confident Answer | 650 | Attacks reveal the target's last position for 2 s, 20% chance. UNVERIFIED |
| Prompt Injection | 700 | The next ability the target casts hits its nearest ally, 4 s, 30 s cooldown. UNVERIFIED |
| Sycophant | 800 | Aura 500 radius: allies inside gain Reach +100. UNVERIFIED |
| Drain Blade | 1,600 | 12% attack lifesteal for 6 s after an attack lands. UNVERIFIED |

Costs set the tier bands in the component tree. Costs are final for v1 and change only in the balance pass.
