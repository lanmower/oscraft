const fs = require('fs');
const root = process.argv[2];
const u = fs.readFileSync(root + '/scenarios/moba/units.osd', 'utf8').split('\n');
const tsv = fs.readFileSync(root + '/scenarios/moba/creeps_688g.tsv', 'utf8').split('\n').filter(l => l && !l.startsWith('#'));
const hdr = tsv[0].split('\t');
const rows = new Map(tsv.slice(1).map(l => { const c = l.split('\t'); return [c[0], Object.fromEntries(hdr.map((h, i) => [h, c[i]]))]; }));
const idOf = l => (l.match(/^unit\("(\w+)"/) || [])[1];
const field = (l, k) => { const m = l.match(new RegExp('\\b' + k + '=(-?\\d+)')); return m ? m[1] : null; };
let diffs = 0, compared = 0; const bad = [];
for (const l of u) { const id = idOf(l); if (!id || !rows.has(id)) continue; const v = rows.get(id); const parts = l.match(/^unit\("\w+", (-?\d+), (-?\d+), (-?\d+)/);
  const checks = [['hp', parts && parts[1], v.hp], ['spd', parts && parts[2], v.spd], ['weapon_damage', field(l, 'weapon_damage'), v.damage], ['weapon_range_milli', field(l, 'weapon_range_milli'), v.range_milli], ['weapon_period', field(l, 'weapon_period'), v.period], ['sight_milli', field(l, 'sight_milli'), v.sight_milli], ['health_regen', field(l, 'health_regen'), v.regen_milli]];
  if (v.armor) checks.push(['defense', field(l, 'defense'), v.armor]);
  for (const [k, got, want] of checks) { if (want === '' || want === undefined) continue; compared++; if (String(got) !== String(want)) { diffs++; bad.push(id + ':' + k + ' ' + got + '!=' + want); } }
}
console.log('fields compared', compared, 'differ', diffs, bad.slice(0, 12).join(' | '));
const ex = root + '/scenarios/moba/units_extra.osd';
console.log('units_extra.osd exists', fs.existsSync(ex), fs.existsSync(ex) ? fs.readFileSync(ex, 'utf8').split('\n').filter(l => /^unit\("(creep|neutral|roshan)/.test(l)).length : 0);
console.log('units.osd creep/neutral lines (excluding roshan_pit)', u.filter(l => /^unit\("(creep|neutral|roshan)/.test(l) && !/roshan_pit/.test(l)).length);
