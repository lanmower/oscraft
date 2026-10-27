import { execFileSync } from 'node:child_process';
const repo = 'lanmower/oscraft';
const gh = (args) => execFileSync('gh', ['api', ...args], { encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const raw = (p, ref) => execFileSync('gh', ['api', `repos/${repo}/contents/${p}?ref=${ref}`, '-H', 'Accept: application/vnd.github.raw'], { encoding: 'utf8', maxBuffer: 1 << 26 });
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const names = JSON.parse(gh([`repos/${repo}/contents/scenarios/moba?ref=${head}`])).filter((f) => f.name.endsWith('.osd')).map((f) => f.name);
const all = names.map((n) => raw(`scenarios/moba/${n}`, head)).join('\n');
console.log('HEAD', head);
for (const h of ['luna', 'abaddon']) {
  const units = all.split('\n').filter((l) => l.startsWith(`unit("${h}"`)).length;
  const ha = all.match(new RegExp('hero_abilities\\("' + h + '",\\s*\\[([^\\]]*)\\]'));
  console.log(h, 'unit_rows', units, 'hero_abilities', ha ? 'present' : 'absent');
}
