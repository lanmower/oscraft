const fs = require('fs');
const p = process.argv[2];
const q = s => s.split('~').join(String.fromCharCode(96));
let doc = fs.readFileSync(p, 'utf8');
const pairs = [
  [q('creeps and neutrals in ~units.osd~ and ~units_extra.osd~ (77 rows), the 61 unresolved heroes,'),
   q('creeps and neutrals in ~units.osd~ (19 of its 51 creep, neutral and Roshan unit lines have no 6.88g row and keep their values; ~units_extra.osd~ has none), the 58 unresolved heroes,')],
  [q('the DROP set of ~scripts/apply-creeps-688g.py~ (the ten lines are still in ~units.osd~, see section 9).'),
   q('the DROP set of ~scripts/apply-creeps-688g.py~, removed in ~ebbafb25~ (the ten lines are kept in ~units.osd~, see section 9).')],
  [q('(~neutral_warpine_raider~, ~neutral_prowler_acolyte~, ~neutral_prowler_shaman~, ~neutral_granite_golem~, ~neutral_ice_shaman~, ~neutral_frostbitten_golem~, ~neutral_big_thunder_lizard~, ~neutral_small_thunder_lizard~).'),
   q('(~neutral_warpine_raider~, ~neutral_prowler_acolyte~, ~neutral_prowler_shaman~, ~neutral_granite_golem~, ~neutral_ice_shaman~, ~neutral_frostbitten_golem~, ~neutral_big_thunder_lizard~, ~neutral_small_thunder_lizard~), and the 2 Roshan pit lines (~roshan_pit_day~, ~roshan_pit_night~) have no 6.88g row. The 8 neutrals with a blank armour in ~creeps_688g.tsv~ (~neutral_kobold~, ~neutral_kobold_taskmaster~, ~neutral_ogre_magi~, ~neutral_satyr_hellcaller~, ~neutral_satyr_trickster~, ~neutral_dark_troll~, ~neutral_forest_troll_high_priest~, ~neutral_black_drake~) are converted on every other field and keep their current ~defense=~. Counts at origin ~ebbafb25~: ~units.osd~ has 51 creep, neutral and Roshan unit lines (49 without the two Roshan pit lines); 32 have a TSV row, and 0 of their 248 compared fields differ; 19 lines in ~units.osd~ carry an UNVERIFIED comment.')],
];
for (const [a, b] of pairs) { const n = doc.split(a).length - 1; if (n !== 1) { console.error('match count ' + n + ' for: ' + a.slice(0, 70)); process.exit(2); } doc = doc.split(a).join(b); }
fs.writeFileSync(p, doc);
console.log('replaced', pairs.length, 'pairs');
