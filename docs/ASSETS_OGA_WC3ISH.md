# OpenGameArt wc3-ish collection: inventory and import

Source: https://opengameart.org/content/wc3-ish-mod (collection https://opengameart.org/collection/162729, credits file https://opengameart.org/collection/162729/CREDITS.TXT, 187 KB). Credits were read through the fetch port. One item title is redacted.

## Counts

- Items in the credits file: 514
- Accepted as inventory: 270
- Rejected: 244
- Items whose page links were fetched and archives downloaded: 272 (2 items were not used)


## Shipped models (scenarios/moba/assets/models/oga)

Each is a single mesh and primitive (glb.js reads the first primitive only). Six embed their colormap PNG in the GLB (fence, gate, snowman, tower-top, tree-fantasy, tree-snow); rock-flat.glb refers to an external KTX2 texture (`scenarios/moba/assets/models/Textures/rock-color.ktx2`); cactus, stump and tree-pine-small have no texture and use a flat base colour. Sizes are native metres; scale matches existing props (tree.glb 1.41 m tall, rock-a.glb 0.39 m tall).

| prop key | role | file | source pack | size |
|---|---|---|---|---|
| oga_tree_fantasy | tree | models/oga/tree-fantasy.glb | Fantasy Town Kit (Kenney) | 1.02 x 2.41 x 1.02 m |
| oga_tree_snow | tree | models/oga/tree-snow.glb | Holiday Kit (Kenney) | 1.19 x 1.92 x 1.19 m |
| oga_tree_pine_small | tree | models/oga/tree-pine-small.glb | Nature Kit (Kenney) | 0.48 x 0.91 x 0.55 m |
| oga_rock_flat | rock | models/oga/rock-flat.glb | Nature Kit (Kenney) | 0.78 x 0.26 x 1.02 m |
| oga_snowman | prop | models/oga/snowman.glb | Holiday Kit (Kenney) | 1.10 x 1.06 x 0.69 m |
| oga_stump | prop | models/oga/stump.glb | Nature Kit (Kenney) | 0.36 x 0.27 x 0.37 m |
| oga_cactus | prop | models/oga/cactus.glb | Nature Kit (Kenney) | 0.40 x 0.75 x 0.34 m |
| oga_fence | prop | models/oga/fence.glb | Fantasy Town Kit (Kenney) | 0.08 x 0.38 x 1.00 m (one 1 m fence section) |
| oga_gate | structure | models/oga/gate.glb | Castle Kit (Kenney) | 0.15 x 0.91 x 0.66 m |
| oga_tower_top | building | models/oga/tower-top.glb | Castle Kit (Kenney) | 1.00 x 1.35 x 1.00 m |

Already shipped in scenarios/moba/assets/models (same bytes, no new files): tree-large (Castle Kit), tree-high-round (Fantasy Town), rock-large (Fantasy Town), rocks-large (Castle Kit), lantern (Fantasy Town), stall-green (Fantasy Town), tower-base (Castle Kit), plus the earlier Kenney kit imports.

## Not usable, and why

- Blender (.blend, 754 files) and FBX/COLLADA-only archives: no converter is on the machine; a portable Blender 4.2 tarball was downloaded to /tmp but not extracted or used, so these are unconverted.
- OBJ-only packs: no OBJ-to-GLB converter; the loader needs GLB and the OBJ sets carry no textures to embed.
- Multi-primitive glTF (about 780 files, e.g. Kenney Space/Watercraft/Pirate kits): glb.js reads only the first primitive, so a merge step is needed before use. Not done in this run.
- Animated characters and rigged skins (Quaternius, Animated Characters Pack): skinning and animation are not wired for placement; not imported.
- 18 archive downloads timed out or failed at 300 s (e.g. kaykit_dungeon_pack_1.0.zip, rpg_vfx_pack.7z, icons.7z). Not retried in this run.
- Standalone PNG/JPG sprites and icons (456 PNG, 170 JPG): 2D UI or item art; not placed as props and not shipped.
- Dual-option items: see rejected list.

## Items (514)

| # | Title | Author | Decision | Formats |
|---|---|---|---|---|
| 1 | Jinn&#039;s Icons | Jinn | rejected |  |
| 2 | Painterly Spell Icons part 1 | J. W. Bjerk | rejected |  |
| 3 | Painterly Spell Icons part 2 | eleazzaar | rejected |  |
| 4 | Painterly Spell Icons part 3 | J. W. Bjerk | rejected |  |
| 5 | Painterly Spell Icons part 4 | eleazzaar | rejected |  |
| 6 | Epic RPG Equipment | yinakoSGA | rejected |  |
| 7 | Ice Spell Icons | HorrorPen | rejected |  |
| 8 | Spellbound Icon | Mumu | accepted | png,zip |
| 9 | Moderna Graphical Interface | Jorge Avila | rejected |  |
| 10 | Info-box | kindland | accepted | png,psd |
| 11 | Loading_bar | kindland | accepted | psd |
| 12 | Fantasy UI Borders | Kenney | accepted | png,zip |
| 13 | Flare Item Variation (60x60 Only) | Mumu | accepted | png,jpg |
| 14 | OSARE weapon icons | Blarumyrran | accepted | png,zip |
| 15 | RPG Book Icons Pack | CraftPix.net 2D Game Assets | rejected |  |
| 16 | Fantasy RPG  Consumable Item Icons | yinakoSGA | rejected |  |
| 17 | RPG WAR Skills | CraftPix.net 2D Game Assets | rejected |  |
| 18 | Free RPG Skill Icons for Crafter Blacksmith and Gnome | CraftPix.net 2D Game Assets | rejected |  |
| 19 | Free RPG Skill Icons for Knight | CraftPix.net 2D Game Assets | rejected |  |
| 20 | Basic Gems Icon Set | yinakoSGA | rejected |  |
| 21 | Some Modified Assets from Fantasy Icon Pack by Ravenmore | mold | rejected |  |
| 22 | Wyrmsun  - over 900 items | Drummyfish | accepted | png,zip |
| 23 | Health and Mana Potion | Clint Bellanger | rejected |  |
| 24 | Oozey the Slime | Mophs | rejected |  |
| 25 | Undead Mage | Dmitry.Porotnikov | rejected |  |
| 26 | Ghoul Hand | Der_Wisch | accepted | png |
| 27 | PHO Sword | Dmitry.Porotnikov | rejected |  |
| 28 | Fantasy Sword Hand painted/ Unfinished | mactinite | accepted | png,zip |
| 29 | Low Poly Treasure Chest | Dmitry.Porotnikov | rejected |  |
| 30 | Medieval Houses | Glitch Squirrel | rejected |  |
| 31 | Magic and mechanic in-game interface icons | josepzin | rejected |  |
| 32 | Fantasy Icon Pack by Ravenmore | Ravenmore | rejected |  |
| 33 | Fantasy decoration | Ashiroxzer | rejected |  |
| 34 | Golden Items [RIP Upgrade Pack] | Ravenmore | rejected |  |
| 35 | [Outdated/Archive] Fantasy Icon Pack | Ravenmore | rejected |  |
| 36 | Fantasy UI Box | StumpyStrust | accepted | png,zip,7z |
| 37 | Old Unused UI Stuff | StumpyStrust | accepted | png |
| 38 | UI Button | StumpyStrust | accepted | png |
| 39 | GUI window buttons | johndh | rejected |  |
| 40 | Powers Icons | Clint Bellanger | accepted | jpg,png,blend |
| 41 | G.u.i. for some rpg game | Jorge Avila | rejected |  |
| 42 | Armor Icons 32x32 px (painterly) | Scrittl | rejected |  |
| 43 | colored ability icons | random223 | rejected |  |
| 44 | Dungeon Crawler Sprites | Ashiroxzer | rejected |  |
| 45 | 32x32 Icons (FLARE Compatible) | VWolfdog | rejected |  |
| 46 | Inventory GUI | Wyrmheart | accepted | jpg,psd |
| 47 | Inventory UI | Wyrmheart | rejected |  |
| 48 | Random Item Icons 128x128 | Mumu | accepted | png,zip |
| 49 | Mnimap | kindland | accepted | png,zip |
| 50 | Login Box | kindland | accepted | png |
| 51 | Buttonsss | kindland | accepted | jpg,zip |
| 52 | Attack Icons Wesnoth | eleazzaar | rejected |  |
| 53 | Fantasy UI Elements by Ravenmore | Ravenmore | rejected |  |
| 54 | Zeta Engine Ability Icons (Part1) | yiannisd | rejected |  |
| 55 | RPG Game UI | Wyrmheart | accepted | png,psd |
| 56 | UI Elements | kindland | accepted | png,zip |
| 57 | Death Book | kindland | accepted | png,zip |
| 58 | Buttons | kindland | accepted | png |
| 59 | Feather Icons | Mumu | accepted | png,zip |
| 60 | Potions | Clint Bellanger | rejected |  |
| 61 | Papers | Kutejnikov | accepted | jpg,7z |
| 62 | simple map tiles | MELLE | accepted | png |
| 63 | Scrolls | yd | accepted | png,blend |
| 64 | 22 skill icons | Kutejnikov | accepted | jpg,7z |
| 65 | Gems | Kutejnikov | accepted | jpg,7z |
| 66 | Wooden Shield | Kutejnikov | accepted | jpg,7z |
| 67 | Bottle | Kutejnikov | accepted | jpg,7z |
| 68 | Daggers | Kutejnikov | accepted | jpg,7z |
| 69 | Hammers | Kutejnikov | accepted | jpg,7z |
| 70 | Potion Bottle | Kutejnikov | accepted | jpg,7z |
| 71 | 17 rings | Kutejnikov | accepted | jpg,7z |
| 72 | Fantasy swords | Kutejnikov | rejected |  |
| 73 | Watermelon Sword | Kutejnikov | rejected |  |
| 74 | Book | Kutejnikov | accepted | jpg,png,7z |
| 75 | Cethiel&#039;s Weapons 3D | Drummyfish | accepted | png,jpg,zip |
| 76 | PS1 Style RPG Enemies | CptDrunkBear | accepted | png,zip |
| 77 | Cethiel&#039;s Dragon 3D | Drummyfish | accepted | png,zip |
| 78 | Treasure Chest | mastahcez | accepted | jpg,zip |
| 79 | Tomato Box | Kutejnikov | accepted | jpg,7z |
| 80 | 3D male | Drummyfish | accepted | png,blend,obj |
| 81 | Ryzom selected textures | Drummyfish | rejected |  |
| 82 | Wyrmsun / - over 300 items | Drummyfish | rejected |  |
| 83 | Dungeon Crawl Selected Upscale | Drummyfish | accepted | png,zip,jpg |
| 84 | RPG Item collection 3 | Drummyfish | accepted | jpg,zip |
| 85 | RPG Item collection 2 | Drummyfish | accepted | jpg,zip |
| 86 | Kutejnikov&#039;s Weapons 3D | Drummyfish | accepted | png,zip,jpg |
| 87 | Homemade weapons | Kutejnikov | accepted | jpg,7z |
| 88 | Stylized Magic Book | animatedheaven | rejected |  |
| 89 | Macuahuitl | Kutejnikov | accepted | jpg,7z |
| 90 | Chained Demon | Mophs | rejected |  |
| 91 | Goblin animated by Motion capture | Danimal | rejected |  |
| 92 | Adventurer-militia-peasant low poly | Danimal | rejected |  |
| 93 | Stylish plants | Yughues | accepted | jpg,7z |
| 94 | Modular Village | Keith at Fertile Soil Productions | accepted | png,zip |
| 95 | Goblin Shaman and High shaman | Danimal | rejected |  |
| 96 | Blood altar | Danimal | rejected |  |
| 97 | Goblin wolf rider | Danimal | rejected |  |
| 98 | Boar enemies | Danimal | rejected |  |
| 99 | Flying terror | Danimal | rejected |  |
| 100 | Dark Priest | Danimal | rejected |  |
| 101 | Charging Minotaur | Danimal | rejected |  |
| 102 | Roach, game-ready and animated | Danimal | rejected |  |
| 103 | Pixel Dungeons Set1 - Props and stuff | Danimal | rejected |  |
| 104 | 3TD Fantasy Ruins Pack | Ron Kapaun | accepted | jpg,zip |
| 105 | Ring Set - Precious Metals | Clint Bellanger | rejected |  |
| 106 | Skeleton with rig | Gord Goodwin | accepted | jpg,blend |
| 107 | 3D Castle / Dungeon Tileset Extended | rubberduck | accepted | jpg,zip |
| 108 | Free LowPoly Golds | Yughues | accepted | jpg,7z |
| 109 | Mini Library | yd | accepted | png,blend |
| 110 | Eldritch Portal (Model with PBR Textures) | phantasmaCora | rejected |  |
| 111 | Low Poly Portal | HippoDevelopment | accepted | png,zip |
| 112 | way-point circle | Paul Wortmann | accepted | png,zip |
| 113 | Street Lamp | yd | accepted | png,blend |
| 114 | Food Pack | yd | accepted | png,blend |
| 115 | Armor Set | yd | rejected |  |
| 116 | Potions | yd | accepted | png,blend |
| 117 | Three Barrels | yd | accepted | png,blend |
| 118 | Archery Set | yd | accepted | png,blend |
| 119 | Staff | yd | accepted | png,blend |
| 120 | Five Tools | yd | accepted | png,blend |
| 121 | Knife | yd | accepted | png,blend |
| 122 | Bag of coins | yd | accepted | png,blend |
| 123 | Low poly key | yd | accepted | png,blend |
| 124 | Low Poly Nature Pack | sjolle | accepted | png,zip |
| 125 | Low-poly Crossbow | Lamoot | rejected |  |
| 126 | Anglo-Saxons Shield Pack | Mev Lyshkin | rejected |  |
| 127 | Low-poly mushrooms | Blender Foundation / apricot.blender.org | rejected |  |
| 128 | Low-poly Skeleton head | TMPxyz | accepted | jpg,7z |
| 129 | Strange Nature Stuff | Fleurman | accepted | png,zip |
| 130 | Low Poly Elvish Blade | T Allen Studios | rejected |  |
| 131 | Breakfast Set | Fleurman | accepted | png,zip |
| 132 | Low Poly Animated Unicorn | T Allen Studios | rejected |  |
| 133 | Low Poly Knight | DefecateRainbows | accepted | png,blend |
| 134 | Sledgehammer 3D | GGBotNet | accepted | png,zip |
| 135 | Celtic Flame Sword 3D | GGBotNet | accepted | png,zip |
| 136 | Goth female - Fleur du Mal | TiZiana | rejected |  |
| 137 | handpainted stylized sword | rubberduck | accepted | png,zip |
| 138 | Undead male | TiZiana | rejected |  |
| 139 | Skeleton Warrior | Clint Bellanger | rejected |  |
| 140 | Night Demon | nubux | rejected |  |
| 141 | Low Poly House Items 1/2 | sjolle | accepted | png,zip |
| 142 | KayKit: Dungeon Pack 1.0 | Kay Lousberg | accepted | png,zip |
| 143 | Chest [redacted] style | Yughues | accepted | jpg,7z |
| 144 | TIN weeds | Yughues | accepted | jpg,7z |
| 145 | Forest Monster | CDmir | accepted | png,7z |
| 146 | Old Lady | CDmir | accepted | png,blend |
| 147 | Monk | CDmir | accepted | png,blend,jpg |
| 148 | Goblin | CDmir | accepted | png,blend |
| 149 | Food Kit | Kenney | accepted | png,zip |
| 150 | Medieval Containers | Daniel Andersson | accepted | jpg,zip |
| 151 | Food Pack | Clint Bellanger | rejected |  |
| 152 | Food Pack: Cooking | Clint Bellanger | rejected |  |
| 153 | 3D Food Pack | Zsky | rejected |  |
| 154 | Healthy & Junk Food Asset Pack | KloWorks | accepted | png,zip |
| 155 | Animated Characters Pack | quaternius | accepted | png,zip |
| 156 | Pointers: part 5 | yd | accepted | png,svg,xcf |
| 157 | Hammer "Skorpio 2" | yd | rejected |  |
| 158 | Dwarven Cursor | Jinn | rejected |  |
| 159 | Old tower | yd | accepted | png,blend |
| 160 | Tower v2 | yd | accepted | png,blend |
| 161 | Castle Golem | Mophs | rejected |  |
| 162 | Bone Golem | Mophs | rejected |  |
| 163 | Scarecrow Character | Mophs | rejected |  |
| 164 | Imp | Mophs | rejected |  |
| 165 | Demon | Mophs | rejected |  |
| 166 | The Mobile Cactus | Mophs | accepted | png,zip |
| 167 | "The Ward" - Low Poly Character | Mophs | rejected |  |
| 168 | Animated Archer | Clement Wu, Nikolaus & Botanic | rejected |  |
| 169 | Animated Bezerker | Clement Wu, Nikolaus & Botanic | rejected |  |
| 170 | Animated Cerberus | Clement Wu, Nikolaus & Botanic | rejected |  |
| 171 | Animated Diablous | Clement Wu, Nikolaus & Botanic | rejected |  |
| 172 | Animated Female Mage | Clement Wu, Nikolaus & Botanic | rejected |  |
| 173 | Animated Knight | Clement Wu, Nikolaus & Botanic | rejected |  |
| 174 | Animated Mage | Clement Wu, Nikolaus & Botanic | rejected |  |
| 175 | Animated Priest | Clement Wu, Nikolaus & Botanic | rejected |  |
| 176 | Animated Knight (Walk Edit) | lancel | rejected |  |
| 177 | Rock Golem | Dm3d | rejected |  |
| 178 | Iron armor set | Alejandro CG | rejected |  |
| 179 | Vampire Bat (Animated) | rubberduck | accepted | png,zip |
| 180 | LowPoly Animated Monsters | quaternius | accepted | zip |
| 181 | FPS Character Beetle | rohit_n | rejected |  |
| 182 | FPS Character Seal | rohit_n | rejected |  |
| 183 | FPS Character Knight | rohit_n | rejected |  |
| 184 | FPS Character Worm | rohit_n | rejected |  |
| 185 | Modular RPG Characters | System G6 | accepted | png,7z |
| 186 | [Animated] Wolf | CoinCoin | rejected |  |
| 187 | [Animated] Crossbowman Skeleton | CoinCoin | rejected |  |
| 188 | [Animated] Boss | CoinCoin | rejected |  |
| 189 | Medieval Props Pack | System G6 | accepted | png,7z |
| 190 | RPG VFX pack | System G6 | accepted | 7z |
| 191 | Medieval Buildings Pack | System G6 | accepted | png,7z |
| 192 | Medieval Tavern Props Pack | System G6 | accepted | png,7z |
| 193 | RPG Icons Set | System G6 | accepted | png,7z |
| 194 | Medieval Weapon Pack | System G6 | accepted | png,7z,jpg |
| 195 | Graveyard Kit | Kenney | accepted | png,zip |
| 196 | [Animated] Rat | CoinCoin | rejected |  |
| 197 | [Animated] Crab | CoinCoin | rejected |  |
| 198 | Fire/Ice/Stone Golem | umask007 | rejected |  |
| 199 | Retextured Gnarly Tools | Mev Lyshkin | rejected |  |
| 200 | Low Poly RPG Pack | quaternius | accepted | jpg,zip |
| 201 | Dungeon Assets | vlandemart | accepted | png,zip |
| 202 | Low Poly Nature Pack | quaternius | accepted | jpg,zip |
| 203 | Low Poly Food Pack | quaternius | accepted | png,zip |
| 204 | Free Modular Low Poly Dungeon Pack | rgsdev | accepted | png,zip |
| 205 | Human Basemeshes | pistachio | accepted | png,zip |
| 206 | Undead Squirrel (ANIMATED) | CDmir | accepted | png,zip |
| 207 | LowPoly Medieval Weapons | quaternius | accepted | jpg,zip |
| 208 | Demon | Grefuntor | rejected |  |
| 209 | Angel Shield Effect | Cethiel | accepted | png,zip |
| 210 | Animations - Blood, Hit and Both :D | Sinestesia | accepted | png |
| 211 | Unity Hit VFX Volume 2 | HighPixel | accepted | png,zip |
| 212 | LowPoly Crops Pack | quaternius | accepted | png,zip |
| 213 | LowPoly Survival Pack | quaternius | accepted | jpg,zip |
| 214 | Angel Wings Effect | Cethiel | accepted | png,zip |
| 215 | Water Magic Effect | Cethiel | accepted | png,zip |
| 216 | Arcane Magic Effect | Cethiel | accepted | png,zip |
| 217 | Light - Magic Effect | Cethiel | accepted | png,zip |
| 218 | Pure Projectile - Magic Effect | Cethiel | accepted | png,zip |
| 219 | Scroll | PantherOne | rejected |  |
| 220 | Spell Book Pack | piacenti | rejected |  |
| 221 | Stealthy Spell Icon | progformer | rejected |  |
| 222 | Lightning Shock Spell | Clint Bellanger | rejected |  |
| 223 | Random RPG Icons Part 1 | HorrorPen | rejected |  |
| 224 | Elemental Weapons - Effect | Cethiel | accepted | png,zip |
| 225 | Nature - Magic Effect | Cethiel | accepted | png,zip |
| 226 | A few spell icons | Daft-Freak | rejected |  |
| 227 | lightnings | hdst | accepted | png |
| 228 | Icons - Dark Magic | Cethiel | accepted | png,zip |
| 229 | 3D - Infernal Sword | Cethiel | accepted | png,zip |
| 230 | 3D - AXE - Basic | Cethiel | accepted | png,obj |
| 231 | Fire Wrath - Magic Effect | Cethiel | accepted | png,zip |
| 232 | Earth Impact - Magic Effect | Cethiel | accepted | png,zip |
| 233 | Axes - Set 1 | Cethiel | accepted | png,zip |
| 234 | Cosmic Time - Magic Effect | Cethiel | accepted | png,zip |
| 235 | Skill, item and spell icons | pauliuw | accepted | png,7z |
| 236 | RPG Tooltip | itsmars | accepted | jpg,zip |
| 237 | Gauntlet Cursor | itsmars | accepted | jpg,zip,png |
| 238 | RPG Inventory | itsmars | accepted | jpg,zip,png |
| 239 | Ability Highlight | VRS1 | accepted | png |
| 240 | Experience Bar | itsmars | accepted | jpg,zip |
| 241 | Free UI Hologram Interface | Wenrexa | accepted | jpg,zip |
| 242 | Assets Free Laser Bullets Pack 2020 | Wenrexa | accepted | jpg,zip |
| 243 | LowPoly Crystals Pack | iPoly3D | accepted | png,zip |
| 244 | LowPoly Ninja Shurikens | iPoly3D | accepted | png,zip |
| 245 | Lowpoly Nordic Runes Icons | iPoly3D | accepted | png,zip |
| 246 | Basic RPG Item Icons (Free) | yinakoSGA | rejected |  |
| 247 | Medieval Icons / Inventory +284 | Wenrexa | accepted | jpg,zip |
| 248 | 50 RPG Pirate Skill Icons | CraftPix.net 2D Game Assets | rejected |  |
| 249 | RPG Night Elf Skills Icon Pack | CraftPix.net 2D Game Assets | rejected |  |
| 250 | 48 Paladin Skills | CraftPix.net 2D Game Assets | rejected |  |
| 251 | Halloween Icons Pack | CraftPix.net 2D Game Assets | rejected |  |
| 252 | RPG Mushroom Icons | CraftPix.net 2D Game Assets | rejected |  |
| 253 | Fantasy Character Avatars | CraftPix.net 2D Game Assets | rejected |  |
| 254 | Alchemical Ingredient Icons Pack | CraftPix.net 2D Game Assets | rejected |  |
| 255 | SCI-FI Icon Pack Pixel Art | CraftPix.net 2D Game Assets | rejected |  |
| 256 | Different Sci-Fi Item Icons | CraftPix.net 2D Game Assets | rejected |  |
| 257 | 20 Medieval Icons | CraftPix.net 2D Game Assets | rejected |  |
| 258 | Space Objects Icons | CraftPix.net 2D Game Assets | rejected |  |
| 259 | Game Icons Fantasy things | CraftPix.net 2D Game Assets | rejected |  |
| 260 | Game Icons of Fantasy Mage Outfit | CraftPix.net 2D Game Assets | rejected |  |
| 261 | Game Icons of Fantasy Potions Daggers | CraftPix.net 2D Game Assets | rejected |  |
| 262 | 100 glitch icons | rubberduck | accepted | png,zip |
| 263 | Animal Figurines | Clint Bellanger | rejected |  |
| 264 | Medieval Props Pack 1 | Daniel Andersson | accepted | jpg,zip |
| 265 | Skeleton Outlaw 3D Character | KevDev | accepted | png,zip |
| 266 | Low Poly Fantasy Swords | KevDev | accepted | png,zip |
| 267 | Stylised Fantasy Weapons | mastahcez | rejected |  |
| 268 | FLARE Skeleton Mod - Skeletal Occultist | VWolfdog | rejected |  |
| 269 | 3d Pirate Coin | acasas | rejected |  |
| 270 | handpainted style rocks | rubberduck | accepted | png,zip |
| 271 | Crate & Barrel bundle | mastahcez | accepted | jpg,zip |
| 272 | Hand Painted AXE 02 | Price | accepted | png,zip |
| 273 | [W001] Hand Painted - Composite Sword | Price | accepted | png,zip |
| 274 | FANTASY-sword-set | MELLE | accepted | png |
| 275 | spell set | Scribe | accepted | jpg,zip |
| 276 | Potion Bottle | OwlishMedia | accepted | png,zip |
| 277 | FLARE-compatible (ish) item icons | Stephen Challener (Redshrike), Sandstein | rejected |  |
| 278 | FLARE-compatible jewelry | Stephen Challener (Redshrike) | rejected |  |
| 279 | gemmed items | rubberduck | rejected |  |
| 280 | FANTASY-parchment-set | MELLE | accepted | png |
| 281 | FANTASY-magic-set | MELLE | accepted | png |
| 282 | 3D swords lowpol (). | Game Assets | accepted | jpg,zip |
| 283 | +50 models of Environment | Zsky | rejected |  |
| 284 | 3D Animated Units | Zsky | rejected |  |
| 285 | 3D Buidlings | Zsky | rejected |  |
| 286 | Canopus III - Sci-fi Desert Units | heekun | rejected |  |
| 287 | Tree Toon Stylized | FabinhoSC | accepted | png,blend,zip |
| 288 | 29 grounds and walls (and water) (1024x1024) | Mysteryem | rejected |  |
| 289 | Medieval Tools Pack | System G6 | accepted | png,7z |
| 290 | Old Dungeon Characters | Septh | rejected |  |
| 291 | Polygonal Alien Cave Nature Asset Package | Aligned Games | rejected |  |
| 292 | Polygonal Woodlands Nature Asset Package | AlignedGames | rejected |  |
| 293 | PS1 style Caveman 3D model [rigged] | mujtaba-io | accepted | png,zip |
| 294 | PS1 Style RPG Weapons | CptDrunkBear | accepted | png,zip |
| 295 | Animated Creatures PS1 Style | Zsky | rejected |  |
| 296 | Ps1 Trees | Starry Skydancer | accepted | png,7z |
| 297 | RPG Classes with Retro Style (Only Models and Textures) | Zsky | rejected |  |
| 298 | Retro Medieval Kit | Kenney | accepted | png,zip |
| 299 | Low poly 3D Pigeon model (rigged + animated) [untextured] | mujtaba-io | accepted | png,blend |
| 300 | N64 Style Fantasy Guns, Weapons, and Items | CptDrunkBear | accepted | png,zip |
| 301 | Weapons Pack | Zsky | rejected |  |
| 302 | Staffs Pack | Zsky | rejected |  |
| 303 | Swords Pack | Zsky | rejected |  |
| 304 | Trees Pack | Zsky | rejected |  |
| 305 | RPG Items | Zsky | rejected |  |
| 306 | Furniture Pack | Zsky | rejected |  |
| 307 | Pumpkin 3D | GGBotNet | accepted | png,zip |
| 308 | Coconut 3D | GGBotNet | accepted | png,zip |
| 309 | Watercraft Kit | Kenney | accepted | png,zip |
| 310 | Space Kit | Kenney | accepted | png,zip |
| 311 | Castle Kit | Kenney | accepted | png,zip |
| 312 | Nature Kit | Kenney | accepted | png,zip |
| 313 | Pirate Kit | Kenney | accepted | png,zip |
| 314 | Platformer Kit | Kenney | accepted | png,zip |
| 315 | Fantasy Town Kit | Kenney | accepted | png,zip |
| 316 | Tower Defense Kit | Kenney | accepted | png,zip |
| 317 | Hexagon Kit | Kenney | accepted | png,zip |
| 318 | Space Kit (remade) | Kenney | accepted | png,zip |
| 319 | Holiday Kit | Kenney | accepted | png,zip |
| 320 | LowPoly Medieval Buildings | quaternius | accepted | jpg,zip |
| 321 | LowPoly Modular Dungeon Pack | quaternius | accepted | png,zip |
| 322 | Textured Medieval Street Lamps | bobjh | rejected |  |
| 323 | Medieval Small House Interior | AnyRPG | accepted | jpg,blend |
| 324 | LowPoly Animated Knight | quaternius | accepted | zip |
| 325 | Lowpoly Medieval Torture Devices | iPoly3D | accepted | png,zip |
| 326 | LowPoly Medieval Village Pack | quaternius | accepted | jpg,zip |
| 327 | Medieval Props textured | Clint Bellanger | rejected |  |
| 328 | Modular Medieval House Pack 01 | hreikin | accepted | png,zip |
| 329 | Modular Medieval Building Pack | Daniel Andersson | accepted | jpg,zip |
| 330 | 3d walls props | DeadKir | accepted | jpg,zip |
| 331 | Hay Bale | Mish7913 | accepted | jpg,blend,zip |
| 332 | Low Poly Mushroom Kit | AssetQuest | accepted | jpg,zip |
| 333 | Free Lowpoly Pond Kit | AssetQuest | accepted | jpg,zip |
| 334 | Low-Poly Halloween Kit | AssetQuest | accepted | jpg,zip |
| 335 | Walking Chest | bobjh | rejected |  |
| 336 | Spyke trap Low poly *Updated* | Danimal | rejected |  |
| 337 | Dungeon transition | djonvincent | accepted | png,blend |
| 338 | entrance to dungeon | killyoverdrive | rejected |  |
| 339 | Fantasy Sword | djonvincent | accepted | png,blend |
| 340 | Mage Tower | djonvincent | accepted | png,blend |
| 341 | tower | Enetheru | rejected |  |
| 342 | Tower | Paul Wortmann | accepted | png,zip |
| 343 | Water Tower | kindland | accepted | png,zip |
| 344 | BGE House Tower | JBB | accepted | png,blend |
| 345 | Guard Tower | bobjh | rejected |  |
| 346 | Mysterious Monolith | bobjh | rejected |  |
| 347 | Wizard&#039;s Door | bobjh | rejected |  |
| 348 | Dovecote | bobjh | rejected |  |
| 349 | Unseelie Portrait | bobjh | rejected |  |
| 350 | Dwarf Mining Helmet | bobjh | rejected |  |
| 351 | Dragon&#039;s Egg | bobjh | rejected |  |
| 352 | Carinated Stone Jar | bobjh | rejected |  |
| 353 | Medieval Cottage | bobjh | rejected |  |
| 354 | Old Tome 2 | bobjh | rejected |  |
| 355 | ancient ruins 2 | killyoverdrive | rejected |  |
| 356 | Acient ruins | killyoverdrive | rejected |  |
| 357 | tower defense Archery&Sorcery | killyoverdrive | rejected |  |
| 358 | Spiderbot 1.0 | killyoverdrive | rejected |  |
| 359 | monster face | killyoverdrive | rejected |  |
| 360 | Beetle (basic) | killyoverdrive | rejected |  |
| 361 | Dungeon staircase | djonvincent | accepted | png,blend |
| 362 | Mecha Training Dummy | Danimal | rejected |  |
| 363 | Fantasy Training Target | Danimal | rejected |  |
| 364 | Free 3D Castle / Dungeon Tileset | rubberduck | accepted | png,zip |
| 365 | Modular Dungeon 2 - 3D Models | Keith at Fertile Soil Productions | accepted | jpg,zip,png |
| 366 | John Williams Waterhouse Handpainted Set | Colorado Stark | accepted | png,zip |
| 367 | Fantasy RPG Icons | pbmojART | rejected |  |
| 368 | Beast chains | bobjh | rejected |  |
| 369 | Pillory | bobjh | rejected |  |
| 370 | Barrel | KokoGames | rejected |  |
| 371 | 3D dungeon torture devices | Paul Wortmann | accepted | png,zip |
| 372 | Crates Chests Coins and Barrels Collection | mabaci | accepted | jpg,zip |
| 373 | Old Bible 3D | GGBotNet | rejected |  |
| 374 | Fantasy RPG Icons | Drummyfish | accepted | png,zip |
| 375 | Fort [OpenDungeons] | eugeneloza | accepted | jpg,gz |
| 376 | OpenEyes Medial Pack v1 | AzeS | accepted | png,zip |
| 377 | 3d medieval ship | aqrezes | rejected |  |
| 378 | Breakable Boxes | loafbrr_1 | accepted | png,zip,blend |
| 379 | Cleaned-up base human models | Pompei2 | rejected |  |
| 380 | Pinguin (low poly) | STKRudy85 | accepted | png,7z |
| 381 | LowPoly Animated Farm Animal Pack | quaternius | accepted | zip |
| 382 | Animated Animales Low Poly | quaternius | accepted | png,zip |
| 383 | Stone coffin | Yughues | accepted | jpg,7z |
| 384 | Church | yd | accepted | png,blend,xcf |
| 385 | Beisa Oryx (a type of antelope) | Micket | accepted | png,zip |
| 386 | tree | atze | accepted | png,7z |
| 387 | Low-poly spider (Glest) | Wciow and John.d.h | rejected |  |
| 388 | Tentacle monster | Danimal | rejected |  |
| 389 | Antlion | Clint Bellanger | rejected |  |
| 390 | Customizable Pumpkins | mrpoly | accepted | jpg,zip |
| 391 | Old book | Aare | rejected |  |
| 392 | LowPoly Pirate Props | zisongbr | accepted | png,7z |
| 393 | Stylized Low poly Tools | zisongbr | accepted | png,zip |
| 394 | N64 Potions | n64guy | accepted | png,zip |
| 395 | lower poly barrel | Starry Skydancer | accepted | png,blend |
| 396 | RPG Item collection 1 | Drummyfish | accepted | jpg,zip |
| 397 | 32 Low Poly Models | Drummyfish | accepted | jpg,png,blend,obj |
| 398 | Frog Guy | Drummyfish | accepted | png,zip |
| 399 | Potion Bottles | Brett Post Script | rejected |  |
| 400 | Free Photogrammetric Rock | Yughues | accepted | jpg,7z |
| 401 | Free LowPoly Rocks set01 | Yughues | accepted | jpg,7z |
| 402 | Ceramic bear mug | Aare | rejected |  |
| 403 | Low-poly Swords | Lamoot | rejected |  |
| 404 | 3D Horse ( rigged, laking textures ) | _canyon | rejected |  |
| 405 | Hippo | Micket | accepted | png,zip |
| 406 | Deer Low Poly (rigged) | crownjoshua | accepted | png,zip |
| 407 | Wolf | pixelmannen | accepted | png,zip |
| 408 | Low Poly Fox | Mathilde_Lea | rejected |  |
| 409 | Low Poly Wolf | Mathilde_Lea | rejected |  |
| 410 | Low Poly Rat | Mathilde_Lea | rejected |  |
| 411 | Low Poly Rabbit | Mathilde_Lea | rejected |  |
| 412 | Low Poly Pig | xTerryx | accepted | png,blend |
| 413 | Low Poly Cow | xTerryx | accepted | png,blend |
| 414 | Wolf | Mev Lyshkin | rejected |  |
| 415 | Highpoly, lowpoly weapons from Sintel | © copyright Blender Foundation / www.sintel.org | rejected |  |
| 416 | 3D collectable pack | rubberduck | accepted | png,zip |
| 417 | Zeppelin - steampunk style | johndh | rejected |  |
| 418 | Demon Statue - Flare sprite sheets | ryan.dansie | accepted | png,blend |
| 419 | Isometric Tents (Unknown Horizons) 3D | Unknown Horizons | rejected |  |
| 420 | Coastal tent buildings and animated signal fire (Unknown Horizons) 3D | Unknown Horizons | rejected |  |
| 421 | Tents: Hunter, Lumberjack, Pavilion, Storage (Unknown Horizons) 3D | Unknown Horizons | rejected |  |
| 422 | Shuriken | Clint Bellanger | accepted | jpg,zip |
| 423 | Low-Poly Mushrooms | Clint Bellanger | rejected |  |
| 424 | Wood Apple Crate | Clint Bellanger | rejected |  |
| 425 | Little Lamp | U.R.U. project | rejected |  |
| 426 | Hand Cart lowpoly | WeaponGuy | rejected |  |
| 427 | Low-spec Reticulans | surt | accepted | png,blend |
| 428 | Ogre lowpoly | nubux | rejected |  |
| 429 | Tree Stump | Clint Bellanger | rejected |  |
| 430 | Light crystal | U.R.U. project | rejected |  |
| 431 | Low-poly Wilderness | surt | accepted | jpg,blend |
| 432 | Dagger - Rogue&#039;s Talon | Quandtum | rejected |  |
| 433 | Pallet | Quandtum | accepted | jpg,zip |
| 434 | Game Crates | Quandtum | accepted | jpg,zip |
| 435 | Magic Staff Set | AnyRPG | accepted | jpg,zip |
| 436 | Crafting items | FacadeGaikan | accepted | png,zip |
| 437 | Treasure Pack | mehrasaur | accepted | png,zip |
| 438 | Low poly pirate pack | quaternius | accepted | png,zip |
| 439 | Izchak&#039;s Torches | cemkalyoncu | rejected |  |
| 440 | Toon Styled Stuff | DREAM_SEARCH_REPEAT | accepted | png,zip |
| 441 | [2D] Wooden Box | Cpt_Flash | accepted | png |
| 442 | Animated water texture 128px | YCbCr | accepted | png,zip |
| 443 | Tower | Kutejnikov | rejected |  |
| 444 | Wooden Log | PantherOne | rejected |  |
| 445 | 3D Desert | 2DPIXX | rejected |  |
| 446 | Golden Gamer Collection - Yellow | FacadeGaikan | accepted | png,blend |
| 447 | Baubles | FacadeGaikan | accepted | png,zip |
| 448 | Fast Food | FacadeGaikan | accepted | png,zip |
| 449 | Medieval House Pack | Daniel Andersson | accepted | png,jpg,zip |
| 450 | Wizard | AnthonyMyers | rejected |  |
| 451 | GrubCrawler | AnthonyMyers | rejected |  |
| 452 | free mine assets pack | rubberduck | accepted | png,zip |
| 453 | simple 3d crystals (unwrapped, simple texture) | rubberduck | accepted | png,zip |
| 454 | Old Street Lamp | RDT | rejected |  |
| 455 | Spell icon collection - part 3 | WAR&#039;s Gauntlet team | rejected |  |
| 456 | Weapon Icons 32x32px (painterly) | Scrittl | rejected |  |
| 457 | Health and Manabars - 100x12px & 200x24px | Scrittl | rejected |  |
| 458 | Game Icons of Fantasy Potions Pack 1 | CraftPix.net 2D Game Assets | rejected |  |
| 459 | RPG GUI contstruction kit vMoon | Under the moon | rejected |  |
| 460 | Magic Buttons | Dakal | rejected |  |
| 461 | Multicolor low-res GUI | gecko666 | rejected |  |
| 462 | Buttons variations | yd | accepted | png,xcf,svg |
| 463 | RPG GUI construction kit v1.0 | Lamoot | rejected |  |
| 464 | Spells icons | elerya | rejected |  |
| 465 | Wesnoth attack icons | fmunoz.geo | rejected |  |
| 466 | Flare buttons | Blarumyrran | accepted | png |
| 467 | Knight&#039;s Glove Mouse Cursor + | XOR | rejected |  |
| 468 | Seven Kingdoms | Enlight Software | rejected |  |
| 469 | RPG Cursor Icons Set | System G6 | accepted | png,7z |
| 470 | GUI Buttons Vol.1 | looneybits | accepted | png,zip |
| 471 | Various GUI Elements | brightretro | rejected |  |
| 472 | Various GUI Elements - Part 2 | brightretro | rejected |  |
| 473 | UI for RPG Maker VX Ace | XCVG | accepted | png,zip |
| 474 | Icons, Old Page, Sketch | HorrorPen | rejected |  |
| 475 | Fiery Icon Pack | mmmcgill1232 | rejected |  |
| 476 | Casual Game Button Pack | pzUH | accepted | jpg,zip,svg |
| 477 | Medieval Game Button Pack | pzUH | accepted | jpg,zip,svg |
| 478 | Fantasy Book | yd | accepted | png,xcf |
| 479 | RPG Items | Buch | accepted | png |
| 480 | GUI Windows constructor | yd | accepted | png |
| 481 | Gods and Idols: Building images | APIStudios | rejected |  |
| 482 | Modified And Cliped Magic, Skill & Item Icons | pauliuw | accepted | jpg,png,zip |
| 483 | Arkana-FTS GUI | moOshiE and Pompei2 | rejected |  |
| 484 | Low Poly Desert Castle | T Allen Studios | rejected |  |
| 485 | HoMM 3 inspired icons | Mev Lyshkin | rejected |  |
| 486 | Natural resource deposits and mines (Unknown Horizons) 3D | Unknown Horizons | rejected |  |
| 487 | free 3d-platformer houses | rubberduck | accepted | png,zip |
| 488 | Townhall, Povray Scene File | Varkalandar | rejected |  |
| 489 | Modular Medieval House Pack 04 | hreikin | accepted | png,zip |
| 490 | Medieval Church | Daniel Andersson | accepted | jpg,zip |
| 491 | Medieval Tavern | Daniel Andersson | accepted | jpg,zip |
| 492 | Simple house | Tartos | rejected |  |
| 493 | Medieval house | marmigliorini | rejected |  |
| 494 | Medieval Farm House | T Allen Studios | rejected |  |
| 495 | Nordic Watch Tower | T Allen Studios | rejected |  |
| 496 | KayKit: Medieval Builder Pack 1.0 | Kay Lousberg | accepted | png,zip |
| 497 | Wall, grass, rock, stone, wood and dirt (480) | West | accepted | jpg,zip |
| 498 | Grass textures, tileable | BlueRatDesigns | rejected |  |
| 499 | LowPoly Pinetree | Aredon | accepted | png,zip |
| 500 | Low-Poly Bridge | Mish7913 | rejected |  |
| 501 | Ancient Civ. in the Jungle - 200+ Seamless textures | DevEarley | accepted | png,zip |
| 502 | Handpainted RPG icons | LudicArts | rejected |  |
| 503 | Aztec Buildings | Kuvrot | accepted | jpg,zip |
| 504 | House lowpoly fbx | knik1985 | rejected |  |
| 505 | N64 Stair Texture Template | Emerald Eel Entertainment | accepted | png,zip |
| 506 | Prototype Textures (32px) | Emerald Eel Entertainment | accepted | png,zip |
| 507 | Low-poly tower (Glest) | johndh | rejected |  |
| 508 | Fist Club | Ouren | rejected |  |
| 509 | Poleaxe and Shield | Ouren | rejected |  |
| 510 | Le Grimoire de Coeur | Ouren | rejected |  |
| 511 | UI resources | Wyrmheart | accepted | jpg,psd |
| 512 | 700+ RPG Icons | Lorc | rejected |  |
| 513 | TIN RPG - Dragon Regalia GUI | tiopalada | accepted | png,zip |
| 514 | Health and Mana Bars | lancel | rejected |  |
