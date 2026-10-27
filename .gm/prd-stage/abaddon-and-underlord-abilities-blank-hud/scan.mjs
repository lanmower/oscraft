import { execFileSync } from 'node:child_process';
const repo = 'lanmower/oscraft';
const gh = (args) => execFileSync('gh', ['api', ...args], { encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const names = JSON.parse(gh([`repos/${repo}/contents/scenarios/moba?ref=${head}`])).filter((f) => f.name.endsWith('.osd')).map((f) => f.name);
console.log('HEAD', head);
for (const n of names) {
  const t = execFileSync('gh', ['api', `repos/${repo}/contents/scenarios/moba/${n}?ref=${head}`, '-H', 'Accept: application/vnd.github.raw'], { encoding: 'utf8', maxBuffer: 1 << 26 });
  for (const l of t.split('\n')) if (/abaddon|underlord/.test(l)) console.log(n + ': ' + l.slice(0, 260));
}
