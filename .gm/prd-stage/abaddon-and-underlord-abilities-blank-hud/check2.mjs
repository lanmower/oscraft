import { execFileSync } from 'node:child_process';
const repo = 'lanmower/oscraft';
const gh = (args) => execFileSync('gh', ['api', ...args], { encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const raw = (p, ref) => execFileSync('gh', ['api', `repos/${repo}/contents/${p}?ref=${ref}`, '-H', 'Accept: application/vnd.github.raw'], { encoding: 'utf8', maxBuffer: 1 << 26 });
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const names = JSON.parse(gh([`repos/${repo}/contents/scenarios/moba?ref=${head}`])).filter((f) => f.name.endsWith('.osd')).map((f) => f.name);
const all = names.map((n) => raw(`scenarios/moba/${n}`, head)).join('\n');
const heroes = ['abaddon', 'abyssal_underlord'];
let fail = 0;
console.log('HEAD', head, 'OSD', names.length);
for (const h of heroes) {
  const units = all.split('\n').filter((l) => l.startsWith(`unit("${h}"`)).length;
  const ha = all.match(new RegExp('hero_abilities\\("' + h + '",\\s*\\[([^\\]]*)\\]'));
  const abil = ha ? [...ha[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]) : [];
  const undefinedAbil = abil.filter((a) => (all.match(new RegExp('^ability\\("' + a + '"', 'gm')) || []).length !== 1);
  console.log(h, 'unit_rows', units, 'hero_abilities', abil.length, 'undefined_or_dup', undefinedAbil.length, abil.join(','));
  if (units !== 1 || abil.length === 0 || undefinedAbil.length) fail++;
}
console.log(fail === 0 ? 'RESULT: PASS' : 'RESULT: FAIL');
