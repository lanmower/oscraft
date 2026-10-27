# 17. Presentation and readability
Every gameplay signal passes three tests: it reads in grayscale, at gameplay zoom, and at minimap size.

### Size targets

| Element | Author size | Minimum display | Test |
| --- | --- | --- | --- |
| Ability icon | 128 × 128 PNG | 32 px | Motif identifiable in grayscale. Fills 60 to 75% of frame. Bottom-right 25% clear for cooldown numbers. |
| Item icon | 88 × 64 PNG; 176 × 128 master | 44 × 32 | One object, ¾ view, rim light, no text |
| Status icon | 64 × 64 | 18 px | Single glyph in system color with 2 px dark outline |
| Minimap hero icon | 32 × 32 (64 master) | 16 px | Head-only caricature reads as a solid silhouette |
| Hero model | Gameplay zoom, 60 to 90 px tall |  | Silhouette identifies hero and facing, 90% or better in a blind test |
| Value groups |  |  | At least 3 per hero. Head and prop at least 30% luminance delta from body. |
| UI text contrast |  |  | 4.5:1 or better on HUD panels |
| Status glyph against health bar |  |  | 3:1 or better |
| Cast point | 0.2 to 0.5 s |  | Anticipation pose readable top-down. Unique per ability. |
| Death animation | 2.0 s or less |  | |
| AoE telegraph | Team-colored decal |  | Outline at true radius. Fill at 25% opacity or less. |

### Hero shape language

| Shape | Meaning | Detail zone |
| --- | --- | --- |
| Circle | Support, friendly | Head, shoulders and signature prop carry about 70% of detail and saturation. Lower body is low-saturation rest. |
| Square | Durable | Same detail rule. Chest and shoulders carry the read. |
| Triangle | Aggressive, mobile | Silhouette point (weapon, hood, hair) carries the read. |

### Color system

| Meaning | Color | Notes |
| --- | --- | --- |
| Ally team, Morning Feed | #2E8BFF | Light-value structures. Teams also differ in value. |
| Enemy team, Late Night Feed | #FF8A1F | Dark-value structures |
| Self | White and gold outline | Yellow health bar |
| Clout, all Clout UI and VFX | #A35CFF | Reserved for Clout only |
| LIVE | Violet dot #6A1FD1, white ring | Never red. Deeper than Clout's #A35CFF so the two stay apart. |
| Damage and danger | #D9534F | Flashing elements stay below R/(R+G+B) of 0.8 |
| Heal and buff | #3DDC97 | Paired with an upward chevron |
| Debuff | Shape only | Inverted chevron |

Rules:

- Every color signal carries a shape. Ally creeps have rounded hoods. Enemy creeps have angular hoods.
- Colorblind presets: deuteranopia, protanopia, tritanopia, and a custom picker.
- Red is never used for a non-damage signal. LIVE and Clout are never red.

### UI

- Top bar: Clout bar and follower count. Abbreviated 1.2K and 3.4M, exact on hover.
- Scoreboard: K, D, A, Clout, Net Worth. Tooltips: Engagement and Impressions.
- Tooltips: mechanical text first, italic flavor second. 14 px minimum.
- Cursor: four states (default, attack, ability target, invalid). 32 × 32.
- Minimap: Trend timer and Go Viral overlays.
