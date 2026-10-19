# OpenGameArt wc3-ish collection: licence inventory and import

Source: https://opengameart.org/content/wc3-ish-mod (collection https://opengameart.org/collection/162729, credits file https://opengameart.org/collection/162729/CREDITS.TXT, 187 KB). Credits were read through the fetch port.

## Counts

- Items in the credits file: 514
- CC0 only (accepted as inventory): 270
- Rejected: 244 (CC-BY, CC-BY-SA, GPL, OGA-BY, or a dual licence with a non-CC0 option)
- CC0 items whose page links were fetched and archives downloaded: 272 (the 2 dual-licence CC0 items were not used)

Rejection reasons: attribution (CC-BY, OGA-BY) and share-alike or copyleft (CC-BY-SA, GPL) terms would require credits or licence notices in the shipped game; these are excluded so the shipped assets carry no attribution obligations. Dual-licence items that also offer CC0 are excluded to keep one unambiguous licence per asset.

## Shipped models (js/client/models/oga)

Each is a single mesh and primitive (glb.js reads the first primitive only) with its colormap PNG embedded in the GLB (no external texture). Sizes are native metres; scale matches existing doodads (tree.glb 1.41 m tall, rock-a.glb 0.6 m).

| doodad key | role | file | source pack | size |
|---|---|---|---|---|
| oga_tree_fantasy | tree | models/oga/tree-fantasy.glb | Fantasy Town Kit (Kenney, CC0) | 1.02 x 2.41 x 1.02 m |
| oga_tree_snow | tree | models/oga/tree-snow.glb | Holiday Kit (Kenney, CC0) | 1.19 x 1.92 x 1.19 m |
| oga_tree_pine_small | tree | models/oga/tree-pine-small.glb | Nature Kit (Kenney, CC0) | 0.48 x 0.91 x 0.55 m |
| oga_rock_flat | rock | models/oga/rock-flat.glb | Nature Kit (Kenney, CC0) | 0.78 x 0.26 x 1.02 m |
| oga_snowman | prop | models/oga/snowman.glb | Holiday Kit (Kenney, CC0) | 1.10 x 1.06 x 0.69 m |
| oga_stump | prop | models/oga/stump.glb | Nature Kit (Kenney, CC0) | 0.36 x 0.27 x 0.37 m |
| oga_cactus | prop | models/oga/cactus.glb | Nature Kit (Kenney, CC0) | 0.40 x 0.75 x 0.34 m |
| oga_fence | prop | models/oga/fence.glb | Fantasy Town Kit (Kenney, CC0) | 0.08 x 0.38 x 1.00 m (one 1 m fence section) |
| oga_gate | structure | models/oga/gate.glb | Castle Kit (Kenney, CC0) | 0.15 x 0.91 x 0.66 m |
| oga_tower_top | building | models/oga/tower-top.glb | Castle Kit (Kenney, CC0) | 1.00 x 1.35 x 1.00 m |

Already shipped in js/client/models (same bytes, no new files): tree-large (Castle Kit), tree-high-round (Fantasy Town), rock-large (Fantasy Town), rocks-large (Castle Kit), lantern (Fantasy Town), stall-green (Fantasy Town), tower-base (Castle Kit), plus the earlier Kenney kit imports.

## Not usable, and why

- Blender (.blend, 754 files) and FBX/COLLADA-only archives: no converter is on the machine; a portable Blender download was started but was too slow to finish in this run, so these are unconverted.
- OBJ-only packs: no OBJ-to-GLB converter; the loader needs GLB and the OBJ sets carry no textures to embed.
- Multi-primitive glTF (about 780 files, e.g. Kenney Space/Watercraft/Pirate kits): glb.js reads only the first primitive, so a merge step is needed before use. Not done in this run.
- Animated characters and rigged skins (Quaternius, Animated Characters Pack): skinning and animation are not wired for placement; not imported.
- 18 archive downloads timed out or failed at 300 s (e.g. kaykit_dungeon_pack_1.0.zip, rpg_vfx_pack.7z, icons.7z). Not retried in this run.
- Standalone PNG/JPG sprites and icons (456 PNG, 170 JPG): 2D UI or item art; not placed as doodads and not shipped.
- Dual-licence items: see rejected list.

## Items (514)

| # | Title | Author | Licence | Decision | Reason or formats |
|---|---|---|---|---|---|
| 1 | Jinn&#039;s Icons | Jinn | CC-BY-SA 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0, GPL 2.0) |
| 2 | Painterly Spell Icons part 1 | J. W. Bjerk | CC-BY 3.0; CC-BY-SA 4.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 4.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 3 | Painterly Spell Icons part 2 | eleazzaar | CC-BY 3.0; CC-BY-SA 4.0; CC-BY-SA 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 4.0, CC-BY-SA 3.0, GPL 2.0) |
| 4 | Painterly Spell Icons part 3 | J. W. Bjerk | CC-BY 3.0; CC-BY-SA 4.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 4.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 5 | Painterly Spell Icons part 4 | eleazzaar | CC-BY 3.0; CC-BY-SA 4.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 4.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 6 | Epic RPG Equipment | yinakoSGA | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 7 | Ice Spell Icons | HorrorPen | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 8 | Spellbound Icon | Mumu | CC0 | accepted | png,zip |
| 9 | Moderna Graphical Interface | Jorge Avila | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 10 | Info-box | kindland | CC0 | accepted | png,psd |
| 11 | Loading_bar | kindland | CC0 | accepted | psd |
| 12 | Fantasy UI Borders | Kenney | CC0 | accepted | png,zip |
| 13 | Flare Item Variation (60x60 Only) | Mumu | CC0 | accepted | png,jpg |
| 14 | OSARE weapon icons | Blarumyrran | CC0 | accepted | png,zip |
| 15 | RPG Book Icons Pack | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 16 | Fantasy RPG  Consumable Item Icons | yinakoSGA | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 17 | RPG Warlock Skills | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 18 | Free RPG Skill Icons for Crafter Blacksmith and Gnome | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 19 | Free RPG Skill Icons for Knight | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 20 | Basic Gems Icon Set | yinakoSGA | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 21 | Some Modified Assets from Fantasy Icon Pack by Ravenmore | mold | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 22 | Wyrmsun CC0 - over 900 items | Drummyfish | CC0 | accepted | png,zip |
| 23 | Health and Mana Potion | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 24 | Oozey the Slime | Mophs | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 25 | Undead Mage | Dmitry.Porotnikov | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 26 | Ghoul Hand | Der_Wisch | CC0 | accepted | png |
| 27 | Phoenix Sword | Dmitry.Porotnikov | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 28 | Fantasy Sword Hand painted/ Unfinished | mactinite | CC0 | accepted | png,zip |
| 29 | Low Poly Treasure Chest | Dmitry.Porotnikov | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 30 | Medieval Houses | Glitch Squirrel | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 31 | Magic and mechanic in-game interface icons | josepzin | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 32 | Fantasy Icon Pack by Ravenmore | Ravenmore | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 33 | Fantasy decoration | Ashiroxzer | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 34 | Golden Items [RIP Upgrade Pack] | Ravenmore | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 35 | [Outdated/Archive] Fantasy Icon Pack | Ravenmore | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 36 | Fantasy UI Box | StumpyStrust | CC0 | accepted | png,zip,7z |
| 37 | Old Unused UI Stuff | StumpyStrust | CC0 | accepted | png |
| 38 | UI Button | StumpyStrust | CC0 | accepted | png |
| 39 | GUI window buttons | johndh | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 40 | Powers Icons | Clint Bellanger | CC0 | accepted | jpg,png,blend |
| 41 | G.u.i. for some rpg game | Jorge Avila | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 42 | Armor Icons 32x32 px (painterly) | Scrittl | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0) |
| 43 | colored ability icons | random223 | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 44 | Dungeon Crawler Sprites | Ashiroxzer | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 45 | 32x32 Icons (FLARE Compatible) | VWolfdog | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 46 | Inventory GUI | Wyrmheart | CC0 | accepted | jpg,psd |
| 47 | Inventory UI | Wyrmheart | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 48 | Random Item Icons 128x128 | Mumu | CC0 | accepted | png,zip |
| 49 | Mnimap | kindland | CC0 | accepted | png,zip |
| 50 | Login Box | kindland | CC0 | accepted | png |
| 51 | Buttonsss | kindland | CC0 | accepted | jpg,zip |
| 52 | Attack Icons Wesnoth | eleazzaar | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 53 | Fantasy UI Elements by Ravenmore | Ravenmore | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 54 | Zeta Engine Ability Icons (Part1) | yiannisd | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 55 | RPG Game UI | Wyrmheart | CC0 | accepted | png,psd |
| 56 | UI Elements | kindland | CC0 | accepted | png,zip |
| 57 | Death Book | kindland | CC0 | accepted | png,zip |
| 58 | Buttons | kindland | CC0 | accepted | png |
| 59 | Feather Icons | Mumu | CC0 | accepted | png,zip |
| 60 | Potions | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 61 | Papers | Kutejnikov | CC0 | accepted | jpg,7z |
| 62 | simple map tiles | MELLE | CC0 | accepted | png |
| 63 | Scrolls | yd | CC0 | accepted | png,blend |
| 64 | 22 skill icons | Kutejnikov | CC0 | accepted | jpg,7z |
| 65 | Gems | Kutejnikov | CC0 | accepted | jpg,7z |
| 66 | Wooden Shield | Kutejnikov | CC0 | accepted | jpg,7z |
| 67 | Bottle | Kutejnikov | CC0 | accepted | jpg,7z |
| 68 | Daggers | Kutejnikov | CC0 | accepted | jpg,7z |
| 69 | Hammers | Kutejnikov | CC0 | accepted | jpg,7z |
| 70 | Potion Bottle | Kutejnikov | CC0 | accepted | jpg,7z |
| 71 | 17 rings | Kutejnikov | CC0 | accepted | jpg,7z |
| 72 | Fantasy swords | Kutejnikov | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 73 | Watermelon Sword | Kutejnikov | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 74 | Book | Kutejnikov | CC0 | accepted | jpg,png,7z |
| 75 | Cethiel&#039;s Weapons 3D | Drummyfish | CC0 | accepted | png,jpg,zip |
| 76 | PS1 Style RPG Enemies | CptDrunkBear | CC0 | accepted | png,zip |
| 77 | Cethiel&#039;s Dragon 3D | Drummyfish | CC0 | accepted | png,zip |
| 78 | Treasure Chest | mastahcez | CC0 | accepted | jpg,zip |
| 79 | Tomato Box | Kutejnikov | CC0 | accepted | jpg,7z |
| 80 | 3D male | Drummyfish | CC0 | accepted | png,blend,obj |
| 81 | Ryzom selected textures | Drummyfish | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 82 | Wyrmsun CC-BY-SA/GPL - over 300 items | Drummyfish | GPL 2.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, CC-BY-SA 3.0) |
| 83 | Dungeon Crawl Selected Upscale | Drummyfish | CC0 | accepted | png,zip,jpg |
| 84 | RPG Item collection 3 | Drummyfish | CC0 | accepted | jpg,zip |
| 85 | RPG Item collection 2 | Drummyfish | CC0 | accepted | jpg,zip |
| 86 | Kutejnikov&#039;s Weapons 3D | Drummyfish | CC0 | accepted | png,zip,jpg |
| 87 | Homemade weapons | Kutejnikov | CC0 | accepted | jpg,7z |
| 88 | Stylized Magic Book | animatedheaven | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 89 | Macuahuitl | Kutejnikov | CC0 | accepted | jpg,7z |
| 90 | Chained Demon | Mophs | CC-BY 3.0; GPL 3.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 3.0, GPL 3.0, OGA-BY 3.0) |
| 91 | Goblin animated by Motion capture | Danimal | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 92 | Adventurer-militia-peasant low poly | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 93 | Stylish plants | Yughues | CC0 | accepted | jpg,7z |
| 94 | Modular Village | Keith at Fertile Soil Productions | CC0 | accepted | png,zip |
| 95 | Goblin Shaman and High shaman | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 96 | Blood altar | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 97 | Goblin wolf rider | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 98 | Boar enemies | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 99 | Flying terror | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 100 | Dark Priest | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 101 | Charging Minotaur | Danimal | CC-BY 4.0; CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 4.0, CC-BY 3.0) |
| 102 | Roach, game-ready and animated | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 103 | Pixel Dungeons Set1 - Props and stuff | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 104 | 3TD Fantasy Ruins Pack | Ron Kapaun | CC0 | accepted | jpg,zip |
| 105 | Ring Set - Precious Metals | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 106 | Skeleton with rig | Gord Goodwin | CC0 | accepted | jpg,blend |
| 107 | 3D Castle / Dungeon Tileset Extended | rubberduck | CC0 | accepted | jpg,zip |
| 108 | Free LowPoly Golds | Yughues | CC0 | accepted | jpg,7z |
| 109 | Mini Library | yd | CC0 | accepted | png,blend |
| 110 | Eldritch Portal (Model with PBR Textures) | phantasmaCora | CC-BY 4.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 4.0, GPL 3.0) |
| 111 | Low Poly Portal | HippoDevelopment | CC0 | accepted | png,zip |
| 112 | way-point circle | Paul Wortmann | CC0 | accepted | png,zip |
| 113 | Street Lamp | yd | CC0 | accepted | png,blend |
| 114 | Food Pack | yd | CC0 | accepted | png,blend |
| 115 | Armor Set | yd | GPL 2.0 | rejected | GPL/copyleft (GPL 2.0) |
| 116 | Potions | yd | CC0 | accepted | png,blend |
| 117 | Three Barrels | yd | CC0 | accepted | png,blend |
| 118 | Archery Set | yd | CC0 | accepted | png,blend |
| 119 | Staff | yd | CC0 | accepted | png,blend |
| 120 | Five Tools CC0 | yd | CC0 | accepted | png,blend |
| 121 | Knife | yd | CC0 | accepted | png,blend |
| 122 | Bag of coins | yd | CC0 | accepted | png,blend |
| 123 | Low poly key | yd | CC0 | accepted | png,blend |
| 124 | Low Poly Nature Pack | sjolle | CC0 | accepted | png,zip |
| 125 | Low-poly Crossbow | Lamoot | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 126 | Anglo-Saxons Shield Pack | Mev Lyshkin | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 127 | Low-poly mushrooms | (c) copyright Blender Foundation / apricot.blender.org | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 128 | Low-poly Skeleton head | TMPxyz | CC0 | accepted | jpg,7z |
| 129 | Strange Nature Stuff | Fleurman | CC0 | accepted | png,zip |
| 130 | Low Poly Elvish Blade | T Allen Studios | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 131 | Breakfast Set | Fleurman | CC0 | accepted | png,zip |
| 132 | Low Poly Animated Unicorn | T Allen Studios | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 133 | Low Poly Knight | DefecateRainbows | CC0 | accepted | png,blend |
| 134 | Sledgehammer 3D | GGBotNet | CC0 | accepted | png,zip |
| 135 | Celtic Flame Sword 3D | GGBotNet | CC0 | accepted | png,zip |
| 136 | Goth female - Fleur du Mal | TiZiana | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 137 | handpainted stylized sword | rubberduck | CC0 | accepted | png,zip |
| 138 | Undead male | TiZiana | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 139 | Skeleton Warrior | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 140 | Night Demon | nubux | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 141 | Low Poly House Items 1/2 | sjolle | CC0 | accepted | png,zip |
| 142 | KayKit: Dungeon Pack 1.0 | Kay Lousberg | CC0 | accepted | png,zip |
| 143 | Chest Blizzard style | Yughues | CC0 | accepted | jpg,7z |
| 144 | Tiny weeds | Yughues | CC0 | accepted | jpg,7z |
| 145 | Forest Monster | CDmir | CC0 | accepted | png,7z |
| 146 | Old Lady | CDmir | CC0 | accepted | png,blend |
| 147 | Monk | CDmir | CC0 | accepted | png,blend,jpg |
| 148 | Goblin | CDmir | CC0 | accepted | png,blend |
| 149 | Food Kit | Kenney | CC0 | accepted | png,zip |
| 150 | Medieval Containers | Daniel Andersson | CC0 | accepted | jpg,zip |
| 151 | Food Pack | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 152 | Food Pack: Cooking | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 153 | 3D Food Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 154 | Healthy & Junk Food Asset Pack | KloWorks | CC0 | accepted | png,zip |
| 155 | Animated Characters Pack | quaternius | CC0 | accepted | png,zip |
| 156 | Pointers: part 5 | yd | CC0 | accepted | png,svg,xcf |
| 157 | Hammer "Skorpio 2" | yd | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 158 | Dwarven Cursor | Jinn | CC-BY-SA 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0, GPL 2.0) |
| 159 | Old tower | yd | CC0 | accepted | png,blend |
| 160 | Tower v2 | yd | CC0 | accepted | png,blend |
| 161 | Castle Golem | Mophs | CC-BY 3.0; GPL 3.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 3.0, GPL 3.0, OGA-BY 3.0) |
| 162 | Bone Golem | Mophs | CC-BY 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, GPL 3.0) |
| 163 | Scarecrow Character | Mophs | CC-BY 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, GPL 3.0) |
| 164 | Imp | Mophs | CC-BY 3.0; GPL 3.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 3.0, GPL 3.0, OGA-BY 3.0) |
| 165 | Demon | Mophs | CC-BY 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, GPL 3.0) |
| 166 | The Mobile Cactus | Mophs | CC0 | accepted | png,zip |
| 167 | "The Ward" - Low Poly Character | Mophs | CC-BY 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, GPL 3.0) |
| 168 | Animated Archer | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 169 | Animated Bezerker | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 170 | Animated Cerberus | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 171 | Animated Diablous | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 172 | Animated Female Mage | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 173 | Animated Knight | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 174 | Animated Mage | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 175 | Animated Priest | Clement Wu, Nikolaus & Botanic | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 176 | Animated Knight (Walk Edit) | lancel | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0) |
| 177 | Rock Golem | Dm3d | CC-BY 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0) |
| 178 | Iron armor set | Alejandro CG | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 179 | Vampire Bat (Animated) | rubberduck | CC0 | accepted | png,zip |
| 180 | LowPoly Animated Monsters | quaternius | CC0 | accepted | zip |
| 181 | FPS Character Beetle | rohit_n | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 182 | FPS Character Seal | rohit_n | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 183 | FPS Character Knight | rohit_n | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 184 | FPS Character Worm | rohit_n | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 185 | Modular RPG Characters | System G6 | CC0 | accepted | png,7z |
| 186 | [Animated] Wolf | CoinCoin | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 187 | [Animated] Crossbowman Skeleton | CoinCoin | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 188 | [Animated] Boss | CoinCoin | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 189 | Medieval Props Pack | System G6 | CC0 | accepted | png,7z |
| 190 | RPG VFX pack | System G6 | CC0 | accepted | 7z |
| 191 | Medieval Buildings Pack | System G6 | CC0 | accepted | png,7z |
| 192 | Medieval Tavern Props Pack | System G6 | CC0 | accepted | png,7z |
| 193 | RPG Icons Set | System G6 | CC0 | accepted | png,7z |
| 194 | Medieval Weapon Pack | System G6 | CC0 | accepted | png,7z,jpg |
| 195 | Graveyard Kit | Kenney | CC0 | accepted | png,zip |
| 196 | [Animated] Rat | CoinCoin | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 197 | [Animated] Crab | CoinCoin | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 198 | Fire/Ice/Stone Golem | umask007 | CC-BY 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0) |
| 199 | Retextured Gnarly Tools | Mev Lyshkin | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 200 | Low Poly RPG Pack | quaternius | CC0 | accepted | jpg,zip |
| 201 | Dungeon Assets | vlandemart | CC0 | accepted | png,zip |
| 202 | Low Poly Nature Pack | quaternius | CC0 | accepted | jpg,zip |
| 203 | Low Poly Food Pack | quaternius | CC0 | accepted | png,zip |
| 204 | Free Modular Low Poly Dungeon Pack | rgsdev | CC0 | accepted | png,zip |
| 205 | Human Basemeshes | pistachio | CC0 | accepted | png,zip |
| 206 | Undead Squirrel (ANIMATED) | CDmir | CC0 | accepted | png,zip |
| 207 | LowPoly Medieval Weapons | quaternius | CC0 | accepted | jpg,zip |
| 208 | Demon | Grefuntor | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 209 | Angel Shield Effect | Cethiel | CC0 | accepted | png,zip |
| 210 | Animations - Blood, Hit and Both :D | Sinestesia | CC0 | accepted | png |
| 211 | Unity Hit VFX Volume 2 | HighPixel | CC0 | accepted | png,zip |
| 212 | LowPoly Crops Pack | quaternius | CC0 | accepted | png,zip |
| 213 | LowPoly Survival Pack | quaternius | CC0 | accepted | jpg,zip |
| 214 | Angel Wings Effect | Cethiel | CC0 | accepted | png,zip |
| 215 | Water Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 216 | Arcane Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 217 | Light - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 218 | Pure Projectile - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 219 | Scroll | PantherOne | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 220 | Spell Book Pack | piacenti | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 221 | Stealthy Spell Icon | progformer | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 222 | Lightning Shock Spell | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 223 | Random RPG Icons Part 1 | HorrorPen | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 224 | Elemental Weapons - Effect | Cethiel | CC0 | accepted | png,zip |
| 225 | Nature - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 226 | A few spell icons | Daft-Freak | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 227 | lightnings | hdst | CC0 | accepted | png |
| 228 | Icons - Dark Magic | Cethiel | CC0 | accepted | png,zip |
| 229 | 3D - Infernal Sword | Cethiel | CC0 | accepted | png,zip |
| 230 | 3D - Axe - Basic | Cethiel | CC0 | accepted | png,obj |
| 231 | Fire Wrath - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 232 | Earth Impact - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 233 | Axes - Set 1 | Cethiel | CC0 | accepted | png,zip |
| 234 | Cosmic Time - Magic Effect | Cethiel | CC0 | accepted | png,zip |
| 235 | Skill, item and spell icons | pauliuw | CC0 | accepted | png,7z |
| 236 | RPG Tooltip | itsmars | CC0 | accepted | jpg,zip |
| 237 | Gauntlet Cursor | itsmars | CC0 | accepted | jpg,zip,png |
| 238 | RPG Inventory | itsmars | CC0 | accepted | jpg,zip,png |
| 239 | Ability Highlight | VRS1 | CC0 | accepted | png |
| 240 | Experience Bar | itsmars | CC0 | accepted | jpg,zip |
| 241 | Free UI Hologram Interface | Wenrexa | CC0 | accepted | jpg,zip |
| 242 | Assets Free Laser Bullets Pack 2020 | Wenrexa | CC0 | accepted | jpg,zip |
| 243 | LowPoly Crystals Pack | iPoly3D | CC0 | accepted | png,zip |
| 244 | LowPoly Ninja Shurikens | iPoly3D | CC0 | accepted | png,zip |
| 245 | Lowpoly Nordic Runes Icons | iPoly3D | CC0 | accepted | png,zip |
| 246 | Basic RPG Item Icons (Free) | yinakoSGA | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 247 | Medieval Icons / Inventory +284 | Wenrexa | CC0 | accepted | jpg,zip |
| 248 | 50 RPG Pirate Skill Icons | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 249 | RPG Night Elf Skills Icon Pack | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 250 | 48 Paladin Skills | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 251 | Halloween Icons Pack | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 252 | RPG Mushroom Icons | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 253 | Fantasy Character Avatars | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 254 | Alchemical Ingredient Icons Pack | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 255 | SCI-FI Icon Pack Pixel Art | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 256 | Different Sci-Fi Item Icons | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 257 | 20 Medieval Icons | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 258 | Space Objects Icons | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 259 | Game Icons Fantasy things | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 260 | Game Icons of Fantasy Mage Outfit | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 261 | Game Icons of Fantasy Potions Daggers | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 262 | 100 glitch icons | rubberduck | CC0 | accepted | png,zip |
| 263 | Animal Figurines | Clint Bellanger | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 264 | Medieval Props Pack 1 | Daniel Andersson | CC0 | accepted | jpg,zip |
| 265 | Skeleton Outlaw 3D Character | KevDev | CC0 | accepted | png,zip |
| 266 | Low Poly Fantasy Swords | KevDev | CC0 | accepted | png,zip |
| 267 | Stylised Fantasy Weapons | mastahcez | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 268 | FLARE Skeleton Mod - Skeletal Occultist | VWolfdog | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 269 | 3d Pirate Coin | acasas | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 270 | handpainted style rocks | rubberduck | CC0 | accepted | png,zip |
| 271 | Crate & Barrel bundle | mastahcez | CC0 | accepted | jpg,zip |
| 272 | Hand Painted Axe 02 | Price | CC0 | accepted | png,zip |
| 273 | [W001] Hand Painted - Composite Sword | Price | CC0 | accepted | png,zip |
| 274 | FANTASY-sword-set | MELLE | CC0 | accepted | png |
| 275 | spell set | Scribe | CC0 | accepted | jpg,zip |
| 276 | Potion Bottle | OwlishMedia | CC0 | accepted | png,zip |
| 277 | FLARE-compatible (ish) item icons | Stephen Challener (Redshrike), Sandstein | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 278 | FLARE-compatible jewelry | Stephen Challener (Redshrike) | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0; GPL 2.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0, GPL 2.0, OGA-BY 3.0) |
| 279 | gemmed items | rubberduck | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 280 | FANTASY-parchment-set | MELLE | CC0 | accepted | png |
| 281 | FANTASY-magic-set | MELLE | CC0 | accepted | png |
| 282 | 3D swords lowpol (CC0). | CC0 Game Assets | CC0 | accepted | jpg,zip |
| 283 | +50 models of Environment | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 284 | 3D Animated Units | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 285 | 3D Buidlings | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 286 | Canopus III - Sci-fi Desert Units | heekun | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 287 | Tree Toon Stylized | FabinhoSC | CC0 | accepted | png,blend,zip |
| 288 | 29 grounds and walls (and water) (1024x1024) | Mysteryem | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 289 | Medieval Tools Pack | System G6 | CC0 | accepted | png,7z |
| 290 | Old Dungeon Characters | Septh | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 291 | Polygonal Alien Cave Nature Asset Package | Aligned Games | GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 3.0, CC-BY-SA 3.0) |
| 292 | Polygonal Woodlands Nature Asset Package | AlignedGames | GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 3.0, CC-BY-SA 3.0) |
| 293 | PS1 style Caveman 3D model [rigged] | mujtaba-io | CC0 | accepted | png,zip |
| 294 | PS1 Style RPG Weapons | CptDrunkBear | CC0 | accepted | png,zip |
| 295 | Animated Creatures PS1 Style | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 296 | Ps1 Trees | Starry Skydancer | CC0 | accepted | png,7z |
| 297 | RPG Classes with Retro Style (Only Models and Textures) | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 298 | Retro Medieval Kit | Kenney | CC0 | accepted | png,zip |
| 299 | Low poly 3D Pigeon model (rigged + animated) [untextured] | mujtaba-io | CC0 | accepted | png,blend |
| 300 | N64 Style Fantasy Guns, Weapons, and Items | CptDrunkBear | CC0 | accepted | png,zip |
| 301 | Weapons Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 302 | Staffs Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 303 | Swords Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 304 | Trees Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 305 | RPG Items | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 306 | Furniture Pack | Zsky | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 307 | Pumpkin 3D | GGBotNet | CC0 | accepted | png,zip |
| 308 | Coconut 3D | GGBotNet | CC0 | accepted | png,zip |
| 309 | Watercraft Kit | Kenney | CC0 | accepted | png,zip |
| 310 | Space Kit | Kenney | CC0 | accepted | png,zip |
| 311 | Castle Kit | Kenney | CC0 | accepted | png,zip |
| 312 | Nature Kit | Kenney | CC0 | accepted | png,zip |
| 313 | Pirate Kit | Kenney | CC0 | accepted | png,zip |
| 314 | Platformer Kit | Kenney | CC0 | accepted | png,zip |
| 315 | Fantasy Town Kit | Kenney | CC0 | accepted | png,zip |
| 316 | Tower Defense Kit | Kenney | CC0 | accepted | png,zip |
| 317 | Hexagon Kit | Kenney | CC0 | accepted | png,zip |
| 318 | Space Kit (remade) | Kenney | CC0 | accepted | png,zip |
| 319 | Holiday Kit | Kenney | CC0 | accepted | png,zip |
| 320 | LowPoly Medieval Buildings | quaternius | CC0 | accepted | jpg,zip |
| 321 | LowPoly Modular Dungeon Pack | quaternius | CC0 | accepted | png,zip |
| 322 | Textured Medieval Street Lamps | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 323 | Medieval Small House Interior | AnyRPG | CC0 | accepted | jpg,blend |
| 324 | LowPoly Animated Knight | quaternius | CC0 | accepted | zip |
| 325 | Lowpoly Medieval Torture Devices | iPoly3D | CC0 | accepted | png,zip |
| 326 | LowPoly Medieval Village Pack | quaternius | CC0 | accepted | jpg,zip |
| 327 | Medieval Props textured | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 328 | Modular Medieval House Pack 01 | hreikin | CC0 | accepted | png,zip |
| 329 | Modular Medieval Building Pack | Daniel Andersson | CC0 | accepted | jpg,zip |
| 330 | 3d walls props | DeadKir | CC0 | accepted | jpg,zip |
| 331 | Hay Bale | Mish7913 | CC0 | accepted | jpg,blend,zip |
| 332 | Low Poly Mushroom Kit | AssetQuest | CC0 | accepted | jpg,zip |
| 333 | Free Lowpoly Pond Kit | AssetQuest | CC0 | accepted | jpg,zip |
| 334 | Low-Poly Halloween Kit | AssetQuest | CC0 | accepted | jpg,zip |
| 335 | Walking Chest | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 336 | Spyke trap Low poly *Updated* | Danimal | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 337 | Dungeon transition | djonvincent | CC0 | accepted | png,blend |
| 338 | entrance to dungeon | killyoverdrive | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 339 | Fantasy Sword | djonvincent | CC0 | accepted | png,blend |
| 340 | Mage Tower | djonvincent | CC0 | accepted | png,blend |
| 341 | tower | Enetheru | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 342 | Tower | Paul Wortmann | CC0 | accepted | png,zip |
| 343 | Water Tower | kindland | CC0 | accepted | png,zip |
| 344 | BGE House Tower | JBB | CC0 | accepted | png,blend |
| 345 | Guard Tower | bobjh | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 346 | Mysterious Monolith | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 347 | Wizard&#039;s Door | bobjh | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 348 | Dovecote | bobjh | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 349 | Unseelie Portrait | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 350 | Dwarf Mining Helmet | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 351 | Dragon&#039;s Egg | bobjh | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 352 | Carinated Stone Jar | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 353 | Medieval Cottage | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 354 | Old Tome 2 | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 355 | ancient ruins 2 | killyoverdrive | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 356 | Acient ruins | killyoverdrive | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 357 | tower defense Archery&Sorcery | killyoverdrive | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 358 | Spiderbot 1.0 | killyoverdrive | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 359 | monster face | killyoverdrive | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 360 | Beetle (basic) | killyoverdrive | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 361 | Dungeon staircase | djonvincent | CC0 | accepted | png,blend |
| 362 | Mecha Training Dummy | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 363 | Fantasy Training Target | Danimal | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 364 | Free 3D Castle / Dungeon Tileset | rubberduck | CC0 | accepted | png,zip |
| 365 | Modular Dungeon 2 - 3D Models | Keith at Fertile Soil Productions | CC0 | accepted | jpg,zip,png |
| 366 | John Williams Waterhouse Handpainted Set | Colorado Stark | CC0 | accepted | png,zip |
| 367 | Fantasy RPG Icons | pbmojART | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 368 | Beast chains | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 369 | Pillory | bobjh | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 370 | Barrel | KokoGames | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 371 | 3D dungeon torture devices | Paul Wortmann | CC0 | accepted | png,zip |
| 372 | Crates Chests Coins and Barrels Collection | mabaci | CC0 | accepted | jpg,zip |
| 373 | Old Bible 3D | GGBotNet | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 374 | Fantasy RPG Icons | Drummyfish | CC0 | accepted | png,zip |
| 375 | Fort [OpenDungeons] | eugeneloza | CC0 | accepted | jpg,gz |
| 376 | OpenEyes Medial Pack v1 | AzeS | CC0 | accepted | png,zip |
| 377 | 3d medieval ship | aqrezes | OGA-BY 3.0; CC0 | rejected | dual licence (CC0 offered with OGA-BY 3.0); kept to one unambiguous licence |
| 378 | Breakable Boxes | loafbrr_1 | CC0 | accepted | png,zip,blend |
| 379 | Cleaned-up base human models | Pompei2 | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 380 | Pinguin (low poly) | STKRudy85 | CC0 | accepted | png,7z |
| 381 | LowPoly Animated Farm Animal Pack | quaternius | CC0 | accepted | zip |
| 382 | Animated Animales Low Poly | quaternius | CC0 | accepted | png,zip |
| 383 | Stone coffin | Yughues | CC0 | accepted | jpg,7z |
| 384 | Church | yd | CC0 | accepted | png,blend,xcf |
| 385 | Beisa Oryx (a type of antelope) | Micket | CC0 | accepted | png,zip |
| 386 | tree | atze | CC0 | accepted | png,7z |
| 387 | Low-poly spider (Glest) | Wciow and John.d.h | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 388 | Tentacle monster | Danimal | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 389 | Antlion | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 390 | Customizable Pumpkins | mrpoly | CC0 | accepted | jpg,zip |
| 391 | Old book | Aare | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 392 | LowPoly Pirate Props | zisongbr | CC0 | accepted | png,7z |
| 393 | Stylized Low poly Tools | zisongbr | CC0 | accepted | png,zip |
| 394 | N64 Potions | n64guy | CC0 | accepted | png,zip |
| 395 | CC0 lower poly barrel | Starry Skydancer | CC0 | accepted | png,blend |
| 396 | RPG Item collection 1 | Drummyfish | CC0 | accepted | jpg,zip |
| 397 | 32 Low Poly Models | Drummyfish | CC0 | accepted | jpg,png,blend,obj |
| 398 | Frog Guy | Drummyfish | CC0 | accepted | png,zip |
| 399 | Potion Bottles | Brett Post Script | CC-BY-SA 4.0 | rejected | requires attribution or share-alike (CC-BY-SA 4.0) |
| 400 | Free Photogrammetric Rock | Yughues | CC0 | accepted | jpg,7z |
| 401 | Free LowPoly Rocks set01 | Yughues | CC0 | accepted | jpg,7z |
| 402 | Ceramic bear mug | Aare | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 403 | Low-poly Swords | Lamoot | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 404 | 3D Horse ( rigged, laking textures ) | _canyon | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 405 | Hippo | Micket | CC0 | accepted | png,zip |
| 406 | Deer Low Poly (rigged) | crownjoshua | CC0 | accepted | png,zip |
| 407 | Wolf | pixelmannen | CC0 | accepted | png,zip |
| 408 | Low Poly Fox | Mathilde_Lea | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 409 | Low Poly Wolf | Mathilde_Lea | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 410 | Low Poly Rat | Mathilde_Lea | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 411 | Low Poly Rabbit | Mathilde_Lea | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 412 | Low Poly Pig | xTerryx | CC0 | accepted | png,blend |
| 413 | Low Poly Cow | xTerryx | CC0 | accepted | png,blend |
| 414 | Wolf | Mev Lyshkin | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 415 | Highpoly, lowpoly weapons from Sintel | © copyright Blender Foundation / www.sintel.org | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 416 | 3D collectable pack | rubberduck | CC0 | accepted | png,zip |
| 417 | Zeppelin - steampunk style | johndh | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 418 | Demon Statue - Flare sprite sheets | ryan.dansie | CC0 | accepted | png,blend |
| 419 | Isometric Tents (Unknown Horizons) 3D | Unknown Horizons | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 420 | Coastal tent buildings and animated signal fire (Unknown Horizons) 3D | Unknown Horizons | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 421 | Tents: Hunter, Lumberjack, Pavilion, Storage (Unknown Horizons) 3D | Unknown Horizons | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 422 | Shuriken | Clint Bellanger | CC0 | accepted | jpg,zip |
| 423 | Low-Poly Mushrooms | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 424 | Wood Apple Crate | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 425 | Little Lamp | U.R.U. project | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 426 | Hand Cart lowpoly | WeaponGuy | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 427 | Low-spec Reticulans | surt | CC0 | accepted | png,blend |
| 428 | Ogre lowpoly | nubux | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 429 | Tree Stump | Clint Bellanger | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 430 | Light crystal | U.R.U. project | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 431 | Low-poly Wilderness | surt | CC0 | accepted | jpg,blend |
| 432 | Dagger - Rogue&#039;s Talon | Quandtum | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 433 | Pallet | Quandtum | CC0 | accepted | jpg,zip |
| 434 | Game Crates | Quandtum | CC0 | accepted | jpg,zip |
| 435 | Magic Staff Set | AnyRPG | CC0 | accepted | jpg,zip |
| 436 | Crafting items | FacadeGaikan | CC0 | accepted | png,zip |
| 437 | Treasure Pack | mehrasaur | CC0 | accepted | png,zip |
| 438 | Low poly pirate pack | quaternius | CC0 | accepted | png,zip |
| 439 | Izchak&#039;s Torches | cemkalyoncu | CC-BY 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0) |
| 440 | Toon Styled Stuff | DREAM_SEARCH_REPEAT | CC0 | accepted | png,zip |
| 441 | [2D] Wooden Box | Cpt_Flash | CC0 | accepted | png |
| 442 | Animated water texture 128px | YCbCr | CC0 | accepted | png,zip |
| 443 | Tower | Kutejnikov | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 444 | Wooden Log | PantherOne | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 445 | 3D Desert | 2DPIXX | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 446 | Golden Gamer Collection - Yellow | FacadeGaikan | CC0 | accepted | png,blend |
| 447 | Baubles | FacadeGaikan | CC0 | accepted | png,zip |
| 448 | Fast Food | FacadeGaikan | CC0 | accepted | png,zip |
| 449 | Medieval House Pack | Daniel Andersson | CC0 | accepted | png,jpg,zip |
| 450 | Wizard | AnthonyMyers | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 451 | GrubCrawler | AnthonyMyers | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 452 | free mine assets pack | rubberduck | CC0 | accepted | png,zip |
| 453 | simple 3d crystals (unwrapped, simple texture) | rubberduck | CC0 | accepted | png,zip |
| 454 | Old Street Lamp | RDT | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 455 | Spell icon collection - part 3 | Warlock&#039;s Gauntlet team | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 456 | Weapon Icons 32x32px (painterly) | Scrittl | CC-BY 3.0; CC-BY-SA 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, CC-BY-SA 3.0, GPL 3.0) |
| 457 | Health and Manabars - 100x12px & 200x24px | Scrittl | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 458 | Game Icons of Fantasy Potions Pack 1 | CraftPix.net 2D Game Assets | OGA-BY 3.0 | rejected | OGA-BY requires attribution (OGA-BY 3.0) |
| 459 | RPG GUI contstruction kit vMoon | Under the moon | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 460 | Magic Buttons | Dakal | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 461 | Multicolor low-res GUI | gecko666 | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 462 | Buttons variations | yd | CC0 | accepted | png,xcf,svg |
| 463 | RPG GUI construction kit v1.0 | Lamoot | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 464 | Spells icons | elerya | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 465 | Wesnoth attack icons | fmunoz.geo | GPL 3.0; GPL 2.0; CC0 | rejected | dual licence (CC0 offered with GPL 3.0, GPL 2.0); kept to one unambiguous licence |
| 466 | Flare buttons | Blarumyrran | CC0 | accepted | png |
| 467 | Knight&#039;s Glove Mouse Cursor + | XOR | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 468 | Seven Kingdoms | Enlight Software | GPL 2.0 | rejected | GPL/copyleft (GPL 2.0) |
| 469 | RPG Cursor Icons Set | System G6 | CC0 | accepted | png,7z |
| 470 | GUI Buttons Vol.1 | looneybits | CC0 | accepted | png,zip |
| 471 | Various GUI Elements | brightretro | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 472 | Various GUI Elements - Part 2 | brightretro | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 473 | CC0 UI for RPG Maker VX Ace | XCVG | CC0 | accepted | png,zip |
| 474 | Icons, Old Page, Sketch | HorrorPen | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 475 | Fiery Icon Pack | mmmcgill1232 | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 476 | Casual Game Button Pack | pzUH | CC0 | accepted | jpg,zip,svg |
| 477 | Medieval Game Button Pack | pzUH | CC0 | accepted | jpg,zip,svg |
| 478 | Fantasy Book | yd | CC0 | accepted | png,xcf |
| 479 | RPG Items | Buch | CC0 | accepted | png |
| 480 | GUI Windows constructor | yd | CC0 | accepted | png |
| 481 | Gods and Idols: Building images | APIStudios | CC-BY 3.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 3.0, OGA-BY 3.0) |
| 482 | Modified And Cliped Magic, Skill & Item Icons | pauliuw | CC0 | accepted | jpg,png,zip |
| 483 | Arkana-FTS GUI | moOshiE and Pompei2 | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 484 | Low Poly Desert Castle | T Allen Studios | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 485 | HoMM 3 inspired icons | Mev Lyshkin | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 486 | Natural resource deposits and mines (Unknown Horizons) 3D | Unknown Horizons | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
| 487 | free 3d-platformer houses | rubberduck | CC0 | accepted | png,zip |
| 488 | Townhall, Povray Scene File | Varkalandar | GPL 3.0; GPL 2.0 | rejected | GPL/copyleft (GPL 3.0, GPL 2.0) |
| 489 | Modular Medieval House Pack 04 | hreikin | CC0 | accepted | png,zip |
| 490 | Medieval Church | Daniel Andersson | CC0 | accepted | jpg,zip |
| 491 | Medieval Tavern | Daniel Andersson | CC0 | accepted | jpg,zip |
| 492 | Simple house | Tartos | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 493 | Medieval house | marmigliorini | GPL 2.0 | rejected | GPL/copyleft (GPL 2.0) |
| 494 | Medieval Farm House | T Allen Studios | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 495 | Nordic Watch Tower | T Allen Studios | CC-BY 4.0 | rejected | requires attribution or share-alike (CC-BY 4.0) |
| 496 | KayKit: Medieval Builder Pack 1.0 | Kay Lousberg | CC0 | accepted | png,zip |
| 497 | Wall, grass, rock, stone, wood and dirt (480) | West | CC0 | accepted | jpg,zip |
| 498 | Grass textures, tileable | BlueRatDesigns | CC-BY 4.0; CC-BY 3.0; CC-BY-SA 4.0; CC-BY-SA 3.0; GPL 3.0; OGA-BY 3.0 | rejected | OGA-BY requires attribution (CC-BY 4.0, CC-BY 3.0, CC-BY-SA 4.0, CC-BY-SA 3.0, GPL 3.0, OGA-BY 3.0) |
| 499 | LowPoly Pinetree | Aredon | CC0 | accepted | png,zip |
| 500 | Low-Poly Bridge | Mish7913 | CC-BY 3.0; GPL 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0, GPL 3.0) |
| 501 | Ancient Civ. in the Jungle - 200+ Seamless textures | DevEarley | CC0 | accepted | png,zip |
| 502 | Handpainted RPG icons | LudicArts | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 503 | Aztec Buildings | Kuvrot | CC0 | accepted | jpg,zip |
| 504 | House lowpoly fbx | knik1985 | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 505 | N64 Stair Texture Template | Emerald Eel Entertainment | CC0 | accepted | png,zip |
| 506 | Prototype Textures (32px) | Emerald Eel Entertainment | CC0 | accepted | png,zip |
| 507 | Low-poly tower (Glest) | johndh | GPL 2.0; GPL 3.0; CC-BY-SA 3.0 | rejected | requires attribution or share-alike (GPL 2.0, GPL 3.0, CC-BY-SA 3.0) |
| 508 | Fist Club | Ouren | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 509 | Poleaxe and Shield | Ouren | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 510 | Le Grimoire de Coeur | Ouren | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 511 | UI resources | Wyrmheart | CC0 | accepted | jpg,psd |
| 512 | 700+ RPG Icons | Lorc | CC-BY 3.0 | rejected | requires attribution or share-alike (CC-BY 3.0) |
| 513 | Tiny RPG - Dragon Regalia GUI | tiopalada | CC0 | accepted | png,zip |
| 514 | Health and Mana Bars | lancel | CC-BY-SA 3.0 | rejected | requires attribution or share-alike (CC-BY-SA 3.0) |
