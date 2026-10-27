# 26. First playable, cut list and checks

The first playable tests what is unique before it tests the classic MOBA basics. Every number here is UNVERIFIED until a build runs the checks below.

## First playable slice

**Goal:** show that Clout, the Algorithm mood and the Trend modifiers make matches different, and that the rest of the game can be built on top.

| Area | In the slice |
| --- | --- |
| Map | Short Form layout (section 14): two lanes per side, the river, both bases, one stage camp per side. |
| Heroes | Six: Based Fiend (Influencer), Lurk Mode (Lurker), Moderator (Moderator), Hot Take (Hot Take), Mentions (Notification Support), Mods (Archivist). One per archetype group. |
| Clout | Earn, decay, Reach, Go Viral, Ratio on death with the 200 cap. |
| Mood | Front Runner, Underdog and Even (section 6). |
| Events | Breaking News, two events per match (section 12); the slice runs the 18:00 one. |
| Modifiers | Two from the pool: Challenge and Learn to Code. |
| Items | Lurker Cam, Spoiler Tag, Anon Mask, Confident Answer. |
| Presentation | LIVE dot, Clout bar, death recap (Ratio), Rebrand record, announcer lines for kills and Ratio. |

**Not in the slice:** Showrunner Draft (draft is a fixed pick order for the slice), the full talent trees, neutral items, Decoy Posts, Safe Spots, Stat Boost, Chat Check, Go Private, Pit Boss, Seasons, spectator reactions, the other 18 heroes.

**Exit criteria:** the runnable checks below pass across at least 200 matches (UNVERIFIED), and the eight playtest scenarios each record a result.

## Cut list

| Item | v1 | Later | Why |
| --- | --- | --- | --- |
| Heroes | 6 | 24 launch, then 112 | Test the mechanics before the roster. |
| Boosts | Algorithm Boost, Bounty, Dank Memes (Haste) | Dank Memes (Illusion, Invisibility, Regeneration), Sweat Mode | Fewer boosts make the Clout effects easier to read. |
| Trend modifiers | 2 | 8 | Compare two before adding more. |
| Talents | None | Full sets from section 21 | Power growth comes after the base kits are tested. |
| Neutral items | None | Section 21 | Tested after camps are proven. |
| Decoy Posts, Safe Spots, Stat Boost | None | Section 21 | Not needed to test Clout. |
| Chat Check | None | Section 21 | Vision is covered by wards. |
| Go Private | None | Section 3 | Building defence is tested later. |
| Repost chain | Yes, cap 5 stacks | Full rules | It is a pillar, so it is included at full cap. |
| Gags | Death recap, Rebrand record, announcer | All others | Gags are presentation, not balance. |
| Seasons and spectator reactions | None | Section 21 | Not part of a test. |

## Playtest scenarios

Run each scenario in the build. Record the result, the time taken and any sign that a rule was unreadable.

1. **Clout comeback.** One team gets 300 Clout ahead for 30 s. Does the Front Runner mood show on the minimap, and does the Underdog catch-up feel fair? Pass: the mood flips and flips back with no flicker inside 30 s.
2. **Lurk ambush.** Lurk Mode waits unseen for 8 s, then uses Unseen Mention. Can a player see from the screen alone that it was unseen? Pass: no LIVE dot, and the state reads without text.
3. **Ratio cap.** A hero dies with 200 or more Clout. Pass: no more than 200 Clout moves, and the split goes to the killer and assists.
4. **Trend modifier fight.** Challenge and Learn to Code both on offer. Pass: the result is logged for the modifier check in section 12.
5. **Breaking News contest.** Both teams have heroes in the zone. Pass: no capture progress is made.
6. **Stage camp.** The same pull is made watched and unwatched. Pass: only the watched pull gives Clout.
7. **Rebrand while Viral.** A Viral hero dies. Pass: the CANCELLED stamp shows, and Rebrand swaps it for Rebranded.
8. **Read check.** Grayscale and each colorblind preset show LIVE, Stan, Ratio and Repost. Pass: each signal is readable without its color.

## Runnable checks

Each check has an ID, an input, an output and a threshold. Thresholds come from section 19.

| ID | Check | Input | Output | Pass |
| --- | --- | --- | --- | --- |
| C1 | Modifier win rate | Matches grouped by picked modifier | Win rate per modifier | 45 to 55% for each |
| C2 | Modifier pick rate | Offers and picks | Share of offers picked | At most 65% |
| C3 | Breaking News contest | Matches with both teams in the zone | Capture progress made | Zero |
| C4 | Mood flicker | Mood flips with timestamps | Flips within 30 s of the last flip | Zero |
| C5 | Ratio cap | Ratio events | Clout moved per death | At most 200 |
| C6 | Disable cap | Hard disables per team | Fourth disable in any 10 s window | Shortened to 0.5 s every time |
| C7 | Snowball | Matches with a 10,000 gold deficit | Comeback rate | At least 10% |
| C8 | Rename regression | Win rate before and after a rename | Difference in points | Within 1 point |
| C9 | Chat mute | Mutes in the first 10 minutes | Share of players | Rotate the phrase pool above 25% |
| C10 | Breaking News capture rate | Captures per event | Capture rate | Track only; no threshold yet |

The Clout-only rule (decision 9 in section 20) is checked by reading the effect text of every Clout effect. It is not a runtime check.
