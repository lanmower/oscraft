# 22. Art direction

Visual direction for the 112 heroes (roster 001 to 111 plus Pit Boss 112), the 96 launch abilities (24 heroes), the 50 item rows in section 11, and the eight Trend modifiers in section 12. Shape, value and color rules come from section 17. Archetypes come from section 15.

## Rules used in every table

- **Body codes.** P1 taller (+10% height). P2 broader (+15% width). P3 compact (-10% height). P4 longer (+15% length). P5 short legs (-20% leg length). P6 big head (+20% head size). All percentages are design targets, UNVERIFIED.
- **Value groups.** Written as body / head / prop, using L (light), M (mid) and D (dark). Head and prop each differ from body by the 30% luminance delta in section 17. Mapping: L about 85%, M about 55%, D about 25% luminance, all UNVERIFIED.
- **Primary value group** is the body value.
- **Sibling rule.** Within an archetype, take the hero's 0-based index k in roster order. Body code is P(k mod 6 + 1). Body value is L, M, D for floor(k/6) = 0, 1, 2. Every prop in an archetype is unique. Any two siblings therefore differ in prop silhouette and in at least one of body code or body value, which meets the two-of-three rule in section 15.
- **Accent shifts** are named steps from the archetype accent. No hex is set here.
- **Color guards.** Keep off #A35CFF (Clout only), red (damage only), #2E8BFF and #FF8A1F (team colors), and #3DDC97 (heal). Hot Take ember and Parasocial rose shift off red. Notification green-cyan shifts off heal green. Ghost lavender stays low in saturation so it does not read as Clout. Influencer violet trim uses the LIVE violet, not the Clout hex.

## 1. Hero table (112)

| # | Hero | Archetype | Distinct prop | Accent shift | Value groups (body / head / prop) |
| --- | --- | --- | --- | --- | --- |
| 001 | Appeals Court (P1) | Moderator | Rolled appeal scroll tied with a wax seal | Cool grey, team tint | L / D / M |
| 002 | Growth Hacker (P1) | Algorithm | Watering can with a funnel spout | Cold teal, lighter | L / D / M |
| 003 | Old Forum Poster (P1) | Ghost | Boxy desktop tower carried like a lunchbox | Pale lavender, grey | L / D / M |
| 004 | Debate Bro (P2) | Algorithm | Clip-on lapel mic on a ribbon | Cold teal, deeper | L / D / D |
| 005 | Yell at Clouds (P1) | Hot Take | Paper rain-cloud on a pole | Desaturated amber, brighter | L / D / M |
| 006 | Bad Night (P1) | Notification | Crescent nightlight on a cord | Soft cyan, cooler | L / D / M |
| 007 | Rideshare Guy (P2) | Hot Take | Car-key fob on a lanyard | Amber, dusty | L / D / D |
| 008 | Community Manager (P2) | Notification | Lanyard stacked with pin badges | Soft cyan, lighter | L / D / M |
| 009 | Bleeding Heart (P3) | Hot Take | Heart-shaped locket with a crack | Muted amber | L / D / M |
| 010 | Retargeting (P1) | Lurker | Binoculars on a strap | Slate, blue | L / D / M |
| 011 | Cold Brew Account (P2) | Moderator | Tall iced-coffee cup with a sleeve | Cool grey, blue-white | L / D / M |
| 012 | Angry Reply (P1) | Reply Guy | Crumpled reply card | Mustard, bright | L / D / M |
| 013 | Spawn Farm (P2) | Ghost | Jar of spiderlings under a mesh lid | Pale lavender, muted | L / D / D |
| 014 | Off-Leash Mod (P3) | Moderator | Loose leash trailing from the wrist | Warm grey, team tint | L / D / D |
| 015 | Unhinged Knight (P4) | Hot Take | Cardboard shield with taped dents | Amber, bronze | L / D / M |
| 016 | Hall Monitor (P3) | Notification | Laminated hall-pass card on a neck cord | Cyan, neutral | L / D / M |
| 017 | Skeleton Account (P3) | Ghost | Skeleton-shaped key on a chain | Lavender-grey, pale | L / D / M |
| 018 | Deadline Bot (P4) | Moderator | Hourglass held in one hand | Grey, team tint | L / D / M |
| 019 | Frost Feed (P1) | Doomscroller | Snow globe held in one hand | Night blue, icy | L / D / M |
| 020 | Night Owl (P2) | Lurker | Clip-on book light | Slate, warm | L / D / D |
| 021 | Read Receipt (P4) | Notification | Folded slip with a tick box, clipped at the hip | Soft cyan, pale | L / D / M |
| 022 | Doomer (P2) | Doomscroller | Half-deflated balloon tied to the wrist | Night blue, greyed | L / D / M |
| 023 | Parental Controls (P5) | Moderator | Keypad slab on the chest | Slate grey, team tint | L / D / D |
| 024 | Doom Thread (P6) | Moderator | Blank placard on a stick | Dim grey, team tint | L / D / M |
| 025 | Dragon Admin (P1) | Moderator | Oversized thumbtack pin on a stick | Steel grey, team tint | M / L / D |
| 026 | Golden Hour (P1) | Influencer | Gold reflector panel | Peach, golden | L / D / M |
| 027 | Rock Star (P2) | Influencer | Guitar strap over the shoulder, no guitar | Peach, bright | L / D / D |
| 028 | Hot Take (P5) | Hot Take | Steaming cup, lid off | Desaturated amber, warm | L / D / M |
| 029 | Lore Keeper (P1) | Archivist | Open ledger held at the chest | Sepia, pale | L / D / M |
| 030 | Spicy Take (P6) | Hot Take | Yellow-ochre chili on a skewer | Amber, pushed warmer | L / D / M |
| 031 | Sweet Talker (P5) | Notification | Bouquet of three flowers | Soft cyan, warm-neutral | L / D / M |
| 032 | Conspiracy Theorist (P2) | Archivist | Corkboard with pinned string on a strap | Sepia, deeper | L / D / D |
| 033 | Unskippable Ad (P2) | Moderator | Boxy portable screen strapped to the chest | Neutral grey, team tint | M / L / D |
| 034 | Drone Footage (P3) | Archivist | Quadcopter hovering at shoulder height | Sepia, cool | L / D / D |
| 035 | Burnout (P1) | Hot Take | Burnt-out tealight in a cup | Grey-amber, ash | M / L / D |
| 036 | Who Asked (P2) | Reply Guy | Question-mark-shaped cane | Mustard, ochre | L / D / D |
| 037 | Wisp Ghostwriter (P6) | Notification | Quill trailing a wisp of smoke | Soft cyan, misty | L / D / D |
| 038 | Dual Monitor (P4) | Archivist | Two hand-held screens stacked | Sepia, tan | L / D / M |
| 039 | Solo Wolf Poster (P3) | Algorithm | Lone candle in a dark jar | Teal-grey, desaturated | L / D / D |
| 040 | Lighting Director (P3) | Influencer | Black flag on a boom arm | Peach, warm | L / D / D |
| 041 | Captain Obvious (P3) | Reply Guy | Billboard sign with one bar | Mustard, warm | L / D / M |
| 042 | Team Lead (P3) | Moderator | Clipboard with a checklist at the hip | Warm grey | M / L / L |
| 043 | Lag Spike (P3) | Doomscroller | Desk lamp on a stalk, one flickering head | Night blue, desaturated | L / D / D |
| 044 | Tenured Lich (P5) | Archivist | Brass name plate on a chain | Sepia, dark | L / D / M |
| 045 | Comment Leech (P1) | Parasocial | Suction-cup plunger on a stick | Rose, dusty | L / D / M |
| 046 | Dragon Scroll (P4) | Doomscroller | Paper kite in the off-hand, string trailing | Night blue, deep | L / D / M |
| 047 | Well Actually (P4) | Reply Guy | Reading glasses on a chain | Mustard, light | L / D / D |
| 048 | Off-Grid Forester (P1) | Hermit | Compass on a cord | Moss, deep | L / D / D |
| 049 | Moonlight Creator (P4) | Influencer | Crescent hand mirror | Peach, cool | L / D / M |
| 050 | Pack Account (P4) | Ghost | Three wolf-head patches on a belt | Lavender-grey, deep | L / D / D |
| 051 | Strong Opinion (P4) | Moderator | Barbell plate on a short bar | Grey, team tint | M / L / D |
| 052 | Ghoster (P3) | Lurker | Card with a torn corner, no reply | Slate, pale | L / D / M |
| 053 | Ban Evader (P5) | Ghost | Four ID cards fanned in one hand | Lavender, cooler | L / D / M |
| 054 | Moonlight Fan (P2) | Parasocial | Crescent glow-stick | Rose, pale | L / D / M |
| 055 | Glow-Up (P5) | Influencer | Star-tipped wand | Peach, pale | L / D / M |
| 056 | Sea Siren (P3) | Parasocial | Shell-handled mirror comb | Rose-mauve, cool | L / D / D |
| 057 | Touch Grass (P2) | Hermit | Dandelion puff in a cupped hand | Sage, light | L / D / M |
| 058 | Old Thread Revival (P6) | Ghost | Dusty hand bell on a bracket | Lavender, dusty | L / D / D |
| 059 | Night Shift (P4) | Lurker | Headlamp | Slate, deep | L / D / M |
| 060 | Stealth Spam (P5) | Lurker | Stack of prepaid cards | Slate, cool | L / D / M |
| 061 | Dual Management (P1) | Notification | Two phones, one in each hand | Soft cyan, mid | M / L / D |
| 062 | Mentions (P2) | Notification | Small service bell on a tray | Soft cyan | M / L / D |
| 063 | Forecaster (P6) | Archivist | Crystal ball on a wooden stand | Sepia, light | L / D / M |
| 064 | Hype Engine (P2) | Hot Take | Pump bellows strapped to the thigh | Amber, brown | M / L / D |
| 065 | Anonymous Poster (P1) | Ghost | Sealed envelope, no return address | Lavender, pale | M / L / D |
| 066 | Multi-Accounter (P2) | Ghost | Fan of ten clip tags | Lavender, bright | M / L / D |
| 067 | Comeback Story (P4) | Parasocial | Taped-up umbrella, ribs held by tape | Rose, faded | L / D / M |
| 068 | Looping Clip (P5) | Parasocial | Film reel slung across the back | Rose, deep | L / D / D |
| 069 | Hook Guy (P1) | Archivist | Fishing gaff hook on a line | Sepia, rust-brown | M / L / D |
| 070 | Late Night Poster (P5) | Doomscroller | Low lantern at the hip, unlit | Night blue, darkest | L / D / M |
| 071 | Screamer (P3) | Hot Take | Dented paper cone at the mouth | Amber, pale | M / L / L |
| 072 | Static Account (P4) | Hot Take | Bundle of frayed cable | Amber, grey | M / L / D |
| 073 | Lurk Mode (P6) | Lurker | Pocket mirror on a rod | Slate, muted | L / D / D |
| 074 | Plagiarist (P3) | Ghost | Paper with a cut-out window, blank | Lavender, grey | M / L / L |
| 075 | Desert Mod (P5) | Moderator | Shovel with a sand pile at the feet | Sand grey, team tint | M / L / L |
| 076 | Shadow Admin (P1) | Lurker | Sealed log book on a chain | Slate, dark | M / L / D |
| 077 | Based Fiend (P6) | Influencer | Folded hand fan with blank slats | Peach, deep | L / D / M |
| 078 | Shadow Agent (P4) | Ghost | Clear acrylic figurine in one palm | Lavender, clear | M / L / D |
| 079 | Shh (P6) | Moderator | Sleep mask pushed up on the forehead | Cool grey | M / L / D |
| 080 | Undead Brand (P5) | Ghost | Name badge with peeling corners | Lavender, faded | M / L / L |
| 081 | Storm Ad (P5) | Hot Take | Rooftop billboard on a pole | Amber, bright | M / L / L |
| 082 | Gacha Whale (P1) | Moderator | Clear capsule with a prize inside, one per hand | Grey, team tint, bright accents | D / L / M |
| 083 | Dropped Connection (P2) | Lurker | Dangling cable with a loose plug | Slate, grey | M / L / D |
| 084 | Long-Form Creator (P1) | Influencer | Long shotgun mic on a boom | Apricot, bright | M / L / D |
| 085 | Haunted Profile (P6) | Ghost | Lantern with a dim flame | Lavender, dark | M / L / D |
| 086 | Content Cop (P2) | Moderator | Torch with a cone of light | Grey, team tint | D / L / M |
| 087 | Storm Chaser (P6) | Hot Take | Rain-gauge cup on a stake | Amber, grey | M / L / D |
| 088 | Ban Hammer (P3) | Moderator | Crate of seized devices on one shoulder | Dark grey, team tint | D / L / M |
| 089 | Mods (P2) | Archivist | Three-day tear-off calendar block | Sepia, grey | M / L / L |
| 090 | Refined Reply (P3) | Lurker | Snap-trap box on a string | Slate, bluer | M / L / D |
| 091 | Hot Mess (P1) | Hot Take | Tipped takeout box, spilled | Amber, ochre | D / L / M |
| 092 | Moderator (P4) | Moderator | Referee whistle on a cord | Grey, team tint | D / L / M |
| 093 | Lumberjack Take (P2) | Hot Take | Felling axe, long handle | Amber, brown | D / L / M |
| 094 | Beta Tester (P3) | Archivist | Flip-up goggles on the forehead | Sepia, bright | M / L / D |
| 095 | Nano Influencer (P2) | Influencer | Tiny clip-on lapel mic | Apricot, pale | M / L / D |
| 096 | Turf Warden (P3) | Hermit | Pruning shears on the belt | Olive, dark | L / D / D |
| 097 | Bait Lord (P5) | Reply Guy | Fishing rod with a bobber | Mustard, pale | L / D / M |
| 098 | Ice Cream Truck (P3) | Notification | Snowball in one hand | Soft cyan, icy | M / L / L |
| 099 | Unkillable Thread (P5) | Moderator | Paper-chain garland looped on the arm | Grey, team tint | D / L / M |
| 100 | Teddy Bear Mode (P3) | Hot Take | Plush teddy bear under the arm | Amber, honey | D / L / M |
| 101 | Grudge Holder (P6) | Parasocial | Ring of old tickets | Rose, muted | L / D / M |
| 102 | Toxic Positivity (P4) | Notification | Oversized sunflower over the shoulder | Soft cyan, sunny | M / L / L |
| 103 | Reviewer Two (P4) | Archivist | Fanned rating cards with stars | Sepia, cream | M / L / L |
| 104 | Mood Board (P3) | Influencer | Rolled poster tube under the arm | Apricot, soft | M / L / L |
| 105 | Ritual Scheduler (P1) | Ghost | Thin candle in a ring holder | Pale lavender, dim | D / L / M |
| 106 | Doomlooper (P4) | Lurker | Loop of rope at the belt | Slate, grey-brown | M / L / L |
| 107 | Wind Vlog (P4) | Influencer | Handheld gimbal on a stick | Apricot, grey | M / L / D |
| 108 | Snow Day (P5) | Notification | Folded sled under one arm | Soft cyan, frosted | M / L / D |
| 109 | Vibe Coach (P6) | Notification | Foam mat rolled under one arm | Soft cyan, muted | M / L / D |
| 110 | Comeback Tour (P1) | Parasocial | Folded flag on a short staff | Rose, bright | M / L / D |
| 111 | Thunder Reply (P4) | Hot Take | Lightning-fork staff | Amber, yellow | D / L / L |
| 112 | Pit Boss (P6) | Moderator | Folding velvet-rope stanchion | Dark grey, team tint | D / L / M |

## 2. Launch abilities (96)

Icon: 128 px, one central motif at 60 to 75% of the frame, bottom-right quarter kept clear for cooldown numbers, at least two value steps so it reads in grayscale, no text. Cast pose: lands inside the 0.2 to 0.5 s cast point and reads top-down. Each pose is unique to its ability.

| Hero | Ability | Icon motif | Cast anticipation pose |
| --- | --- | --- | --- |
| Based Fiend | A1 Quote Post | Three offset speech tabs stepping diagonally, light on dark | Flat rectangle raised to chest, one arm extended forward |
| Based Fiend | A2 Follower Count | Three small circles stacked in a rising column | Cupped hands at the waist, palms up, drawing inward |
| Based Fiend | A3 Vibes Aura | Broken concentric ring around a centre dot | Both arms spread wide and level, palms out |
| Based Fiend | Ult Viral Thread | Looping thread knot ending in an arrowhead | Crouch, then one arm swings a full overhead circle |
| Long-Form Creator | A1 Thread | Wavy line closing into a loop on a dark ground | Drops to one knee, arm sweeps a long low arc |
| Long-Form Creator | A2 Pinned Reply | Pushpin with a speech-tab flag | Hand tucks a small flag behind the shoulder |
| Long-Form Creator | A3 Attention Span | Long bar capped with an eye dot | Arm straight forward, body tall and still |
| Long-Form Creator | Ult Breaking News | Long bar with a wedge point over a bold block | Holds a long flat board overhead in both hands |
| Lurk Mode | A1 Unseen Mention | Curled hook-shaped blade on a dark ground | Crouched low, head down, dagger arm pulled back |
| Lurk Mode | A2 Dark Mode | Dark puff cloud with three small dots | Both arms raised, palms down, body hunched forward |
| Lurk Mode | A3 Cold Open | Dashed arc ending in a sharp point | Body compressed low, arms folded behind the back |
| Lurk Mode | Ult Ghost Post | Outline figure with dashed motion arcs | Hood pulled up, arms crossed over the chest |
| Retargeting | A1 Lead Magnet | Horseshoe magnet with a dot at each tip | Arm cocked behind the shoulder, token held in hand |
| Retargeting | A2 Cold Outreach | Crosshair circle with a tick mark | One hand presses a flat card to the chest, other arm out |
| Retargeting | A3 Stealth Mode | Row of fading footprints | Tiptoe, one foot lifted, arms held close |
| Retargeting | Ult Tracked Post | Pinned crosshair over a coin dot | Index arm points at the target, token pinched between fingers |
| Who Asked | A1 Cold Read | Six-point frost star | Palm raised flat, fingers spread, facing forward |
| Who Asked | A2 Off the Record | Page fading to nothing with a zip line | Hand flat across the mouth, body half-turned away |
| Who Asked | A3 Wall of Text | Stacked horizontal bars with a notch | Both arms sweep low across the front, palms down |
| Who Asked | Ult Hot Take Drop | Teardrop falling into a ring target | Arms raised overhead, then both hands drop together |
| Well Actually | A1 Citation Needed | Folded tab with three hash marks | Index finger jabs forward, other hand on hip |
| Well Actually | A2 Hard Correction | Diagonal cross-out stroke over a round blob | Pointer stick held forward at chest height |
| Well Actually | A3 Drain Thread | Spool with a downward arrow | Both hands cupped in front, drawing inward |
| Well Actually | Ult Source? | Open page under a tilted magnifier | Page held flat in both hands, magnifier tilted over it |
| Debate Bro | A1 Rebuttal | Two opposed arrows meeting at a spark | Fists raised in a guard, feet wide apart |
| Debate Bro | A2 Quick Reply | Two overlapping forward arrows | Body pitched forward, lead arm extended |
| Debate Bro | A3 Block Button | Beveled square button | One forearm crossed over the chest, fist at shoulder |
| Debate Bro | Ult Void Post | Square with an empty hole at the centre | Both palms up and open, arms held wide |
| Solo Wolf Poster | A1 Keyboard Smash | Ring of rounded square key blocks | Arms spread, torso starts a full twist |
| Solo Wolf Poster | A2 Get Well Soon | Rounded plus sign on a card | Hands cupped palms-up, head tilted |
| Solo Wolf Poster | A3 Crit Happens | Eight-point starburst | One fist pumps beside the head |
| Solo Wolf Poster | Ult Doomscroll | Vertical bars scrolling past with dots | Arms rolling in a fast figure-eight, shoulders loose |
| Frost Feed | A1 Frost Feed | Ring of small cold puffs | Palms pressed to the ground, arms splayed |
| Frost Feed | A2 Hold Still | Root hook looping around a ring | Hand flat in front, fingers curled into a grip |
| Frost Feed | A3 Dark Mode Aura | Half-moon over radiating dashes | Arms raised in an arch overhead |
| Frost Feed | Ult Load More | Concentric pulse rings from a centre | Crouches low, arms wide to the ground |
| Dragon Scroll | A1 Dragon Scroll | Long rolled scroll with a serrated tail | Rolled scroll swept low across the chest |
| Dragon Scroll | A2 Flash Feed | Flash burst over a clock dial | Wrist rolled, hand hovering above the head |
| Dragon Scroll | A3 Hype Meter | Vertical meter bars filling from the base | Fist raised at the side in a pump |
| Dragon Scroll | Ult Viral Clip | Film frame with one arrow through it | Arms form a rectangle frame in front of the body |
| Read Receipt | A1 Seen Receipt | Double tick inside a circle | One finger taps at waist height |
| Read Receipt | A2 Close Friends Only | Ring of small dots around a shield | Arm curves around an invisible ally |
| Read Receipt | A3 Push Notification | Badge dot with two bounce arcs | Palm flicks upward once |
| Read Receipt | Ult Bad Vibes | Drooping wave inside a square | Both hands drop slowly, palms down |
| Mentions | A1 Public Apology | Split circle, one half dark, one half light | Palms raised outward in an apology gesture |
| Mentions | A2 Dismiss | Speech bubble crossed by a diagonal | Hand sweeps off the shoulder |
| Mentions | A3 Pinned | Pin with a radiating ring | Fist pressed straight down |
| Mentions | Ult Likes | Three stacked upward chevrons | Two-hand clap at chest height, then palms split wide |
| Moderator | A1 Warning | Triangle sign with a bar | Flat triangular plate held up in front |
| Moderator | A2 Appeal | Shield with a curved return arrow | Arm across the chest, flat palm out |
| Moderator | A3 Strike | Flat slab with radiating lines | Flat slab held overhead, face down, ready to drop |
| Moderator | Ult Lockdown | Padlock over a circle | Both hands clasped together in front |
| Ban Hammer | A1 Gavel Slam | Hammer head with an impact star below | Hammer cocked over one shoulder, other hand braced forward |
| Ban Hammer | A2 Remove | Three bars cut by an arc | Arm sweeps horizontally across the front |
| Ban Hammer | A3 Notice | Flat card with two upward chevrons | Card held flat at shoulder height |
| Ban Hammer | Ult Escalate | Stepped staircase rising | One foot steps up, arms lift in stages |
| Anonymous Poster | A1 Anonymous Dagger | Dagger in a sheet fold | Throwing arm back, dagger in hand |
| Anonymous Poster | A2 Rephrase | Page curl with two swapping dots | Hand flicks a card forward |
| Anonymous Poster | A3 Anon Mode | Blank face outline with a dashed edge | Hood pulled up, both hands at the face |
| Anonymous Poster | Ult Final Post | Pale starburst over a bar | Arms cross, then open into a wide V |
| Ban Evader | A1 Fresh Account | Concentric rings of small roots | Palms flat to the ground |
| Ban Evader | A2 New Account | Two identical outlines offset | Arms out, one foot forward |
| Ban Evader | A3 Sock Puppet | Puppet hand with three curled fingers | Hand raised, fingers curled like a puppet |
| Ban Evader | Ult Alt Accounts | Three overlapping circles | Hand splayed, three fingers spread |
| Hot Take | A1 Breaking Ground | Cracked horizontal line over a ground stripe | Fist strikes forward and down |
| Hot Take | A2 Thread Totem | Totem of three stacked rings | Plants a short pole in front |
| Hot Take | A3 Follow-Up | Ring with a point inside | Arms extend outward in a ring, palms raised |
| Hot Take | Ult Echo Chamber | Nested arcs like ripples | Palms cupped at the mouth |
| Yell at Clouds | A1 Yell at Clouds | Cloud outline with sound brackets | Head tilts back, hand cupped at the mouth |
| Yell at Clouds | A2 Engagement | Hook with a curved tail | Hand raised, fingers curled into a hook |
| Yell at Clouds | A3 Clapback | Arrow bouncing back off a wall | One arm raised, palm out, ready to block |
| Yell at Clouds | Ult Cancel | Circle with a diagonal slash | Arm sweeps across the body in one diagonal |
| Mods | A1 Final Warning | Three small triangles in a ring | Hand tosses three tokens to the ground |
| Mods | A2 Locked Thread | Padlock on a thread line | Both arms outstretched holding a line taut |
| Mods | A3 Jump | Arc with a landing dot | Knees bent, arms swung back |
| Mods | Ult Temp Ban | Timer bar with an X | Thumb pressed on a small remote in the palm |
| Hook Guy | A1 Pulled Into Thread | Hook attached to a thread | Arm whips forward holding the hook |
| Hook Guy | A2 Stale | Grey fog swirl inside a dim circle | Palms down, pressing toward the ground |
| Hook Guy | A3 Stacks | Three growing bars | One hand stacked above the other |
| Hook Guy | Ult Edit | Pen nib over a square with a tick | Hands held as if writing in midair |
| Looping Clip | A1 Recycled Content | Circular arrow forming a loop | Arm throws a flat disc from the hip |
| Looping Clip | A2 Scheduled Post | Calendar tile with a burst | Hand taps the wrist |
| Looping Clip | A3 Ghosting | Three pale fading outlines in a trail | Body leans forward, trailing arm behind |
| Looping Clip | Ult Looping Clip | Film frames forming a ring | Arms rotate in a circle overhead |
| Comeback Tour | A1 Teaser | Line with an ember dot at its tip | Fingers flick upward in a short arc |
| Comeback Tour | A2 Engagement Bait | Hook bent inside a ring | Both hands beckon toward the body |
| Comeback Tour | A3 Crit Bait | Starburst with a hook | Arm strikes with a snapping wrist |
| Comeback Tour | Ult Comeback Arc | Arc rising and falling with a return triangle | One full overhead arc, arms rising from the sides |
| Touch Grass | A1 Fresh Air | Ring of leaf shapes | Arms out, palms up, chest open |
| Touch Grass | A2 Leave the House | Door-frame outline with a leaf | Steps through an imaginary frame |
| Touch Grass | A3 Sunlight | Sun disc with rays above a tree outline | Hands raised and opened to the sky |
| Touch Grass | Ult Go Outside | Door arc with rays radiating out | Both palms push outward |
| Turf Warden | A1 Bark Camouflage | Dotted tree outline fading | Crouched, arms folded to blend in |
| Turf Warden | A2 Leech Feed | Root line with a drip | Hand flat on the ground, fingers spread |
| Turf Warden | A3 Wood Shield | Log slice with grain rings | Round shield held in front |
| Turf Warden | Ult Root Network | Branching roots with nodes | Palms on the ground, body splayed wide |

## 3. Items (50)

Object: one object, three-quarter view, rim light, no text (section 17 item icon). Plinth or glow: consumable is a circular plinth, active is a violet edge glow in the LIVE violet (not #A35CFF), passive has none. Section 11 gives no class column, so classes are read from the effect text (see Open items).

| Item | Object | Plinth or glow |
| --- | --- | --- |
| Cheap Dagger | Short plain dagger, tape-wrapped grip, one chipped edge | None (passive) |
| Drafted Note | Spiral notebook with a ribbon bookmark | None (passive) |
| Sticky Note | Square pad of tabs, top sheet curled back | None (passive) |
| Hot Sauce Bottle | Squeeze bottle with a pointed nozzle cap, one side dented | Circular plinth (consumable) |
| Ring Light | Small ring lamp on a clip arm, unlit | None (passive) |
| Envelope | Flat sealed envelope, flap closed | None (passive) |
| Old Hoodie | Folded hoodie with the hood tucked inside | None (passive) |
| Rubber Duck | Small bath duck, bill forward | None (passive) |
| Burner Phone | Cheap flip phone half open, stub antenna | None (passive) |
| Charger | Wall power brick with its cable wound around it | None (passive) |
| Cold Brew | Tall glass jar with a lid and straw | None (passive) |
| Tripod | Three-leg camera stand, legs splayed | None (passive) |
| Lurker Cam | Small pan-tilt camera, lens cap raised, on a short stalk | Circular plinth (consumable, stock) |
| Dust of Receipts | Corked vial of fine glitter dust beside a curled receipt | Violet edge glow (active) |
| Hot Sauce | Tall glass bottle with a dropper stopper | Circular plinth (consumable) |
| Link in Bio Scroll | Rolled parchment tied with a chain-link tag | Circular plinth (consumable) |
| Envelope With Legs | Sealed envelope on two short stub legs | None (passive) |
| Magic Enthusiast | Glass orb in a wire cradle, one spark at the top | None (passive) |
| Spoiler Tag | Hanging tag on a spike, with a punched hole | Circular plinth (consumable, ward) |
| Ragequit Dagger | Short dagger whose grip is a swung-open door handle | Violet edge glow (active) |
| Gem of Subtext | Faceted gem with layered inner facets, scratched | None (passive) |
| Bottle of Copium | Round-bellied flask with a cork, half full | Circular plinth (consumable) |
| Smoke of Deniability | Squat canister with a vented cap and a wisp of smoke | Circular plinth (consumable, stock) |
| Grindset | Kettlebell with a woven grip | None (passive) |
| Gas Station Sushi | Clear clamshell tray with a rice roll and a plastic fork | Circular plinth (consumable) |
| Lo-Fi Beats | Cassette tape standing on its edge | None (passive) |
| Boots of Scrolling | Single tall boot with a spiral groove in the sole | None (passive) |
| Anon Mask | Blank white half-mask with cut eye holes | Violet edge glow (active) |
| Participation Trophy | Squat cup with a rosette ribbon | Circular plinth (consumable, one revive) |
| Heart of Brainrot | Plush heart cushion with visible stitched seams | None (passive) |
| Sleep Mode | Two-position switch plate with a crescent-moon knob | Violet edge glow (active) |
| Reply All | Round polished mirror shield with an etched speech-bubble ring | Violet edge glow (active) |
| Co-Working Space | Single potted succulent in a shallow tray | None (passive) |
| Rescheduled | Wall clock with one hand bent back, a calendar corner tucked under | Violet edge glow (active) |
| Pro Plan | Sealed rosette badge with a gold star | None (passive) |
| Dunk | Hand bell with a felt plug in the clapper | None (passive) |
| Heavenly Feedback | Stage microphone on a bent stand, cable coiled | None (passive) |
| Ghost Mode | Hoop with a sheer veil draped over it | None (passive) |
| Refresh Page | Brass dial ring with one curved arrowhead | Violet edge glow (active) |
| Rapier, Unironic | Long straight rapier with a plain crossguard | None (passive) |
| Dark Mode Shield | Round matte-black shield with a crescent cut-out | Violet edge glow (active) |
| Main Character Energy | Single stage spotlight on a stand, heat haze at the lens | None (passive) |
| Sponsored Style | Two identical hand mirrors, one set in front | Violet edge glow (active) |
| Monetized | Three gold coins fanned out, one with a tally mark | None (passive) |
| Sheep Check | Clipboard with a wool pompom clipped on top | Violet edge glow (active) |
| Lite Plan | Single flat card with a folded corner tab | None (passive) |
| Confident Answer | Speech-bubble card with a bold underline and a raised eyebrow | None (passive) |
| Prompt Injection | Syringe in tiny unreadable lettering, plunger half down | Violet edge glow (active) |
| Sycophant | Small hand puppet giving a thumbs up, gold trim | None (passive) |
| Drain Blade | Curved knife with a cable running to a battery pack | Violet edge glow (active) |

Grayscale confusions:

- **Bottle family** (Hot Sauce Bottle, Hot Sauce, Cold Brew, Bottle of Copium). Separate by silhouette: squeeze nozzle, tall dropper, straw jar, round flask. Flat value will not separate them.
- **Envelope family** (Envelope, Envelope With Legs, Link in Bio Scroll). Legs and the rolled tube carry the difference.
- **Paper family** (Drafted Note, Sticky Note, Spoiler Tag, Lite Plan). Spiral, pad, hanging tag and single card are the only separators. Sticky Note and Lite Plan are both flat.
- **Camera family** (Lurker Cam, Ring Light, Tripod). Box, ring and legs.
- **Invisibility family** (Smoke of Deniability, Anon Mask, Ghost Mode). Canister, mask and veil. Anon Mask and Ghost Mode are both pale, so the violet glow is the only separator between them.
- **Round shields** (Dark Mode Shield, Reply All). Keep the value gap: matte near-black against bright mirror.
- **Dial family** (Sleep Mode, Rescheduled, Refresh Page). Sleep Mode and Dark Mode Shield both use a crescent, so the switch stays flat and rectangular and the shield stays round.
- **Mirror pair** (Sponsored Style, Reply All). Two hand mirrors against one round shield.
- **Blades** (Cheap Dagger, Ragequit Dagger, Rapier, Unironic). Hilt shape decides.
- **Plush pair** (Heart of Brainrot, Sheep Check). Heart cushion against a clipboard with a pompom.

## 4. The Trend costumes (8)

Constant form, shared by all eight costumes: four tapered slabs stacked narrow to wide, a flag crown on top, faceless. The outline never changes. A costume changes only interior marks and values inside that outline. The Trend body uses no #A35CFF and no red. Team color lives in the HUD filter only.

| # | Modifier (section 12) | Costume | Silhouette-constant form and mark |
| --- | --- | --- | --- |
| 1 | Challenge | Chevron Run | Same stack and crown. Each slab face carries a dark chevron band pointing up the stack. Crown holds three pennants inside its own box. |
| 2 | Unboxing | Flap Stack | Same stack and crown. Each slab's top edge is a folded flap line. Crown is one lifted lid corner. |
| 3 | Learn to Code | Tile Grid | Same stack and crown. Slab faces are a dark grid of square tiles with bracket marks. Crown is one square cursor block. |
| 4 | Speedrun | Streak Stack | Same stack and crown. Slab faces carry horizontal dark speed streaks. Crown holds a round stopwatch dial with one hand. |
| 5 | Storytime | Panel Stack | Same stack and crown. Slab faces split into comic-frame bands. Crown is a hinged clapper flap. |
| 6 | Get Ready With Me | Vanity Stack | Same stack and crown. Each slab edge carries a row of round lamp dots. Crown is a hand-mirror disc. |
| 7 | Algorithm Surge | Surge Stack | Same stack and crown. Slab edges carry rising bar-chart steps. Crown is a lightning bolt on a spring. |
| 8 | Doomscroll | Scroll Stack | Same stack and crown. Slabs are layered scroll sheets. Crown is an endless spool reel. |

## Open items

- Turf Warden's A1 is Bark Camouflage. The earlier name came from a franchise and was renamed.
- Item classes (consumable, active, passive) are inferred from effect text. Section 11 does not state them.
- Body percentages and luminance mapping in the rules are design targets, UNVERIFIED, until the palette pass.
