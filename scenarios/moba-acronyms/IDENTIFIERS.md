# Default Dota/MOBA identifiers: acronyms only

Active hero identifiers are the lowercase forms of `scenarios/moba/hero_acronyms.lua` values. Full Dota names and source slugs are historical provenance only.

The default URL resolves to `scenarios/moba-acronyms` because `js/client/pack_load.js` sets `PACK_PARAM` to that path when no `pack` or `package` query parameter is present. `?package=scenarios/moba` remains the non-active historical source pack. `?package=scenarios/bota` remains the opt-in BOTA pack.

Collision rule: `gen-hero-acronyms.mjs` applies `moba_ui.json` overrides first, then its deterministic initialism rule, then the first unused three-letter alternative from the name's flattened letters. This generated pack refuses duplicate active IDs.

## Mapping

| Source Dota ID | Historical display name | Active acronym ID |
| --- | --- | --- |
| `antimage` | Anti-Mage | `am` |
| `axe` | Axe | `axe` |
| `bane` | Bane Elemental | `be` |
| `bloodseeker` | Bloodseeker | `blo` |
| `crystal_maiden` | Crystal Maiden | `cm` |
| `drow_ranger` | Drow Ranger | `dr` |
| `earthshaker` | Earthshaker | `ear` |
| `juggernaut` | Juggernaut | `jug` |
| `mirana` | mirana | `mir` |
| `nevermore` | nevermore | `nev` |
| `morphling` | Morphling | `mor` |
| `phantom_lancer` | Phantom Lancer | `pl` |
| `puck` | puck | `puc` |
| `pudge` | pudge | `pud` |
| `razor` | razor | `raz` |
| `sand_king` | Sand King | `sk` |
| `storm_spirit` | Storm Spirit | `ss` |
| `sven` | sven | `sve` |
| `tiny` | tiny | `tin` |
| `vengefulspirit` | Vengeful Spirit | `vs` |
| `windrunner` | Windrunner | `win` |
| `zuus` | zuus | `zuu` |
| `kunkka` | kunkka | `kun` |
| `lina` | lina | `lin` |
| `lich` | Lich | `lic` |
| `lion` | lion | `lio` |
| `shadow_shaman` | Shadow Shaman | `shs` |
| `slardar` | slardar | `sla` |
| `tidehunter` | Tidehunter | `tid` |
| `witch_doctor` | Witch Doctor | `wd` |
| `riki` | riki | `rik` |
| `enigma` | Enigma | `eni` |
| `tinker` | Tinker | `tik` |
| `sniper` | sniper | `sni` |
| `necrolyte` | Necrolyte | `nec` |
| `warlock` | Warlock | `war` |
| `beastmaster` | Beastmaster | `bea` |
| `queenofpain` | Queen of Pain | `qop` |
| `venomancer` | Venomancer | `ven` |
| `faceless_void` | Faceless Void | `fv` |
| `skeleton_king` | Skeleton King | `wk` |
| `death_prophet` | Death Prophet | `dp` |
| `phantom_assassin` | Phantom Assassin | `pa` |
| `pugna` | pugna | `pug` |
| `templar_assassin` | Templar Assassin | `ta` |
| `viper` | viper | `vip` |
| `luna` | luna | `lun` |
| `dragon_knight` | Dragon Knight | `dk` |
| `dazzle` | dazzle | `daz` |
| `rattletrap` | rattletrap | `rat` |
| `leshrac` | leshrac | `les` |
| `furion` | furion | `fur` |
| `life_stealer` | Lifestealer | `lif` |
| `dark_seer` | Dark Seer | `ds` |
| `clinkz` | clinkz | `cli` |
| `omniknight` | Omniknight | `omn` |
| `enchantress` | Enchantress | `enc` |
| `huskar` | huskar | `hus` |
| `night_stalker` | Night Stalker | `ns` |
| `broodmother` | Broodmother | `bro` |
| `bounty_hunter` | Bounty Hunter | `bh` |
| `weaver` | weaver | `wea` |
| `jakiro` | jakiro | `jak` |
| `batrider` | Batrider | `bat` |
| `chen` | chen | `che` |
| `spectre` | Spectre | `spe` |
| `doom_bringer` | Doom Bringer | `db` |
| `ancient_apparition` | Ancient Apparition | `aa` |
| `ursa` | ursa | `urs` |
| `spirit_breaker` | Spiritbreaker | `spi` |
| `gyrocopter` | Gyrocopter | `gyr` |
| `alchemist` | Alchemist | `alc` |
| `invoker` | Invoker | `inv` |
| `silencer` | Silencer | `sil` |
| `obsidian_destroyer` | Obsidian Destroyer | `od` |
| `lycan` | lycan | `lyc` |
| `brewmaster` | Pandaren Brewmaster | `pb` |
| `shadow_demon` | Shadow Demon | `sd` |
| `lone_druid` | Lone Druid | `ld` |
| `chaos_knight` | Chaos Knight | `ck` |
| `meepo` | meepo | `mee` |
| `treant` | treant | `tre` |
| `ogre_magi` | Ogre Magi | `om` |
| `undying` | Undying | `und` |
| `rubick` | rubick | `rub` |
| `disruptor` | Disruptor | `dis` |
| `nyx_assassin` | nyx_assassin | `nyx` |
| `naga_siren` | Naga Siren | `nag` |
| `keeper_of_the_light` | Keeper of the Light | `kl` |
| `wisp` | wisp | `wis` |
| `visage` | visage | `vis` |
| `slark` | slark | `slr` |
| `medusa` | medusa | `med` |
| `troll_warlord` | Troll Warlord | `tw` |
| `centaur` | centaur | `cen` |
| `magnataur` | Magnataur | `mag` |
| `shredder` | shredder | `shr` |
| `bristleback` | Bristleback | `bri` |
| `tusk` | Tuskarr | `tus` |
| `skywrath_mage` | Skywrath Mage | `sm` |
| `abaddon` | abaddon | `aba` |
| `elder_titan` | elder_titan | `eld` |
| `legion_commander` | Legion Commander | `lc` |
| `ember_spirit` | Ember Spirit | `es` |
| `earth_spirit` | Earth Spirit | `esp` |
| `terrorblade` | terrorblade | `ter` |
| `phoenix` | Phoenix | `pho` |
| `oracle` | Oracle | `ora` |
| `techies` | techies | `tec` |
| `winter_wyvern` | Winter Wyvern | `ww` |
| `arc_warden` | Arc Warden | `aw` |
| `abyssal_underlord` | abyssal_underlord | `aby` |
| `monkey_king` | monkey_king | `mon` |
| `pangolier` | pangolier | `pan` |
| `dark_willow` | dark_willow | `dar` |
| `grimstroke` | grimstroke | `gri` |
| `mars` | mars | `mar` |
| `void_spirit` | void_spirit | `voi` |
| `snapfire` | snapfire | `sna` |
| `hoodwink` | hoodwink | `hoo` |
| `dawnbreaker` | dawnbreaker | `daw` |
| `marci` | marci | `mac` |
| `primal_beast` | primal_beast | `pri` |
| `muerta` | muerta | `mue` |
| `ringmaster` | ringmaster | `rin` |
| `kez` | kez | `kez` |
| `largo` | largo | `lar` |
