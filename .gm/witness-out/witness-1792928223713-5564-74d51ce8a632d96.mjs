import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const fs = require('fs');
const ui = JSON.parse(fs.readFileSync('scenarios/moba/moba_ui.json', 'utf8'));
const findShop = (o) => { if (o && typeof o === 'object') { if (o.tierCosts) return o; for (const k of Object.keys(o)) { const r = findShop(o[k]); if (r) return r; } } return null; };
const shop = findShop(ui);
const edges = shop.tierCosts;
const tiers = shop.tiers;
const ascending = edges.every((e, i) => i === 0 || e > edges[i - 1]);
const rows = fs.readFileSync('scenarios/moba/items.osd', 'utf8').split('\n').map((l) => /^item\("([^"]+)", (\d+)/.exec(l)).filter(Boolean);
const bands = tiers.map(() => 0); let unverified = 0;
for (const m of rows) { const cost = Number(m[2]); if (cost <= 0) { unverified++; continue; } let t = edges.findIndex((e) => cost <= e); if (t < 0) t = edges.length; bands[t]++; }
const total = bands.reduce((a, b) => a + b, 0) + unverified;
const ok = tiers.length === 4 && edges.length === 3 && ascending && bands.every((n) => n > 0) && total === rows.length;
console.log(JSON.stringify({rows: rows.length, bands, unverified, tiers, edges}));
console.log(ok ? 'RESULT: PASS' : 'RESULT: FAIL');
process.exit(ok ? 0 : 1);