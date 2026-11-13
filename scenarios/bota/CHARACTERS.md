# BOTA character layer

The full BOTA character slug is the only active hero identifier. It is used for unit IDs, draft entries, Lua keys, UI keys, model IDs, portrait IDs, catalogue IDs and all character-facing pack references.

Active roster: 24 fullname slugs from `/workspace/bota/ROSTER.md`, slots 001-027 excluding retired slots 002, 003, 011.

Retired slots 002, 003, 011 are excluded from the pack (002 Chad and 011 Karen from the 2026-11-12 design finalization, and 003 Gigachad image-only Reserve). Slots 028-032 are Reserve with no images and are excluded.

The 127 original MOBA hero IDs below are historical source IDs being mapped FROM. They are not active BOTA identifiers.

The `<ACRO>-skinned.glb` paths hold the 26 verified optimized static GLB meshes from hy3d's `bota-ready.json` (256px textures, glTF 2.0, no animation skins), of which the 24 active heroes are packed. The historical `-skinned` suffix is retained for compatibility; rigged animation is not present.

## Acronym assignments

| Slot | Acronym | Model | Portrait |
| --- | --- | --- | --- |
| 001 | neckbeard | nec | neckbeard-skinned | icons/heroes/neckbeard.jpg |
| 004 | wojak | woj | wojak-skinned | icons/heroes/wojak.jpg |
| 005 | bloomer | blo | bloomer-skinned | icons/heroes/bloomer.jpg |
| 006 | tradwife | tra | tradwife-skinned | icons/heroes/tradwife.jpg |
| 007 | boomer | boo | boomer-skinned | icons/heroes/boomer.jpg |
| 008 | coomer | coo | coomer-skinned | icons/heroes/coomer.jpg |
| 009 | gondola | gon | gondola-skinned | icons/heroes/gondola.jpg |
| 010 | spurdo | spu | spurdo-skinned | icons/heroes/spurdo.jpg |
| 012 | howard | how | howard-skinned | icons/heroes/howard.jpg |
| 013 | sanic | san | sanic-skinned | icons/heroes/sanic.jpg |
| 014 | lankdad | lan | lankdad-skinned | icons/heroes/lankdad.jpg |
| 015 | finger_dan | fin | finger_dan-skinned | icons/heroes/finger_dan.jpg |
| 016 | willie | wil | willie-skinned | icons/heroes/willie.jpg |
| 017 | noscope | nos | noscope-skinned | icons/heroes/noscope.jpg |
| 018 | dino | din | dino-skinned | icons/heroes/dino.jpg |
| 019 | ecce | ecc | ecce-skinned | icons/heroes/ecce.jpg |
| 020 | bot_bernard | bot | bot_bernard-skinned | icons/heroes/bot_bernard.jpg |
| 021 | shoop | sho | shoop-skinned | icons/heroes/shoop.jpg |
| 022 | npc | npc | npc-skinned | icons/heroes/npc.jpg |
| 023 | forever | for | forever-skinned | icons/heroes/forever.jpg |
| 024 | cole | col | cole-skinned | icons/heroes/cole.jpg |
| 025 | grey | gre | grey-skinned | icons/heroes/grey.jpg |
| 026 | ghillie | ghi | ghillie-skinned | icons/heroes/ghillie.jpg |
| 027 | gothgirl | got | gothgirl-skinned | icons/heroes/gothgirl.jpg |

## Original ID to acronym mapping

| Original MOBA source ID | Acronym |
| --- | --- |
| abaddon | nec |
| abyssal_underlord | woj |
| alchemist | blo |
| ancient_apparition | tra |
| antimage | boo |
| arc_warden | coo |
| axe | gon |
| bane | spu |
| batrider | how |
| beastmaster | san |
| bloodseeker | lan |
| bounty_hunter | fin |
| brewmaster | wil |
| bristleback | nos |
| broodmother | din |
| centaur | ecc |
| chaos_knight | bot |
| chen | sho |
| clinkz | npc |
| crystal_maiden | for |
| dark_seer | col |
| dark_willow | gre |
| dawnbreaker | ghi |
| dazzle | got |
| death_prophet | nec |
| disruptor | woj |
| doom_bringer | blo |
| dragon_knight | tra |
| drow_ranger | boo |
| earth_spirit | coo |
| earthshaker | gon |
| elder_titan | spu |
| ember_spirit | how |
| enchantress | san |
| enigma | lan |
| faceless_void | fin |
| furion | wil |
| grimstroke | nos |
| gyrocopter | din |
| hoodwink | ecc |
| huskar | bot |
| invoker | sho |
| jakiro | npc |
| juggernaut | for |
| keeper_of_the_light | col |
| kez | gre |
| kunkka | ghi |
| largo | got |
| legion_commander | nec |
| leshrac | woj |
| lich | blo |
| life_stealer | tra |
| lina | boo |
| lion | coo |
| lone_druid | gon |
| luna | spu |
| lycan | how |
| magnataur | san |
| marci | lan |
| mars | fin |
| medusa | wil |
| meepo | nos |
| mirana | din |
| monkey_king | ecc |
| morphling | bot |
| muerta | sho |
| naga_siren | npc |
| necrolyte | for |
| nevermore | col |
| night_stalker | gre |
| nyx_assassin | ghi |
| obsidian_destroyer | got |
| ogre_magi | nec |
| omniknight | woj |
| oracle | blo |
| pangolier | tra |
| phantom_assassin | boo |
| phantom_lancer | coo |
| phoenix | gon |
| primal_beast | spu |
| puck | how |
| pudge | san |
| pugna | lan |
| queenofpain | fin |
| rattletrap | wil |
| razor | nos |
| riki | din |
| ringmaster | ecc |
| rubick | bot |
| sand_king | sho |
| shadow_demon | npc |
| shadow_shaman | for |
| shredder | col |
| silencer | gre |
| skeleton_king | ghi |
| skywrath_mage | got |
| slardar | nec |
| slark | woj |
| snapfire | blo |
| sniper | tra |
| spectre | boo |
| spirit_breaker | coo |
| storm_spirit | gon |
| sven | spu |
| techies | how |
| templar_assassin | san |
| terrorblade | lan |
| tidehunter | fin |
| tinker | wil |
| tiny | nos |
| treant | din |
| troll_warlord | ecc |
| tusk | bot |
| undying | sho |
| ursa | npc |
| vengefulspirit | for |
| venomancer | col |
| viper | gre |
| visage | ghi |
| void_spirit | got |
| warlock | nec |
| weaver | woj |
| windrunner | blo |
| winter_wyvern | tra |
| wisp | boo |
| witch_doctor | coo |
| zuus | gon |
