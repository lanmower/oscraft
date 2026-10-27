import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
const repo = 'lanmower/oscraft';
const dir = '/workspace/oscraft/.gm/prd-stage/abaddon-and-underlord-abilities-blank-hud';
const mode = process.argv[2] || 'check';
const gh = (args, input) => execFileSync('gh', ['api', ...args], { input, encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const raw = (path, ref) => execFileSync('gh', ['api', `repos/${repo}/contents/${path}?ref=${ref}`, '-H', 'Accept: application/vnd.github.raw'], { encoding: 'utf8', maxBuffer: 1 << 26 });
const names = JSON.parse(gh([`repos/${repo}/contents/scenarios/moba?ref=${head}`])).filter((f) => f.name.endsWith('.osd')).map((f) => f.name);
const osd = {};
for (const n of names) osd[n] = raw(`scenarios/moba/${n}`, head);
const heroes = ['abaddon', 'abyssal_underlord'];
const result = (units, abil) => {
  const problems = [];
  for (const h of heroes) {
    const unitLines = units.split('\n').filter((l) => l.startsWith(`unit("${h}"`));
    if (unitLines.length !== 1) { problems.push(`${h}: unit rows ${unitLines.length}`); continue; }
    const m = unitLines[0].match(/abilities=\[([^\]]*)\]/);
    const names2 = m ? [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]) : [];
    if (names2.length === 0) problems.push(`${h}: no abilities list`);
    for (const a of names2) {
      const defs = Object.values(abil).join('\n').split('\n').filter((l) => l.startsWith(`ability("${a}"`)).length;
      if (defs !== 1) problems.push(`${h}: ability ${a} defined ${defs} times`);
    }
  }
  return problems;
};
const unitsAll = Object.values(osd).join('\n');
const abilAll = osd;
const problems = result(osd['units_heroes.osd'] || '', abilAll);
console.log('HEAD', head, 'OSD_FILES', names.length, 'PROBLEMS', problems.length);
for (const p of problems) console.log('  ' + p);
console.log(problems.length === 0 ? 'RESULT: PASS' : 'RESULT: FAIL');
