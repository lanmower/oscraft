import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const fs = require('fs');
const cp = require('child_process');
const REPO = 'lanmower/oscraft';
const gh = (args, input) => cp.execSync('gh ' + args, { encoding: 'utf8', input, maxBuffer: 64 * 1024 * 1024 }).trim();
const chk = cp.spawnSync('node', ['/workspace/oscraft/scripts/check-staged-secrets.mjs'], { encoding: 'utf8', cwd: '/workspace/oscraft' });
if (chk.status !== 0 || !/RESULT: PASS/.test(chk.stdout)) throw new Error('secrets check: ' + chk.status + ' ' + (chk.stdout + chk.stderr).slice(-400));
const login = gh('api user --jq .login');
if (login !== 'lanmower') throw new Error('gh login is ' + login);
const pre = {1:'e258f04cc7',2:'5d30324fe0',3:'5bb2230763',4:'4409d08c51',5:'1d19fd0470',6:'260b39f298',7:'7e8a0161c7',8:'72b9967a32',9:'02e020c485',10:'ee55334dd4',11:'9bac65f689',12:'c7f3a96a3f'};
const head = gh('api repos/' + REPO + '/git/ref/heads/main --jq .object.sha');
const baseTree = gh('api repos/' + REPO + '/git/commits/' + head + ' --jq .tree.sha');
const tree = [];
for (let i = 1; i <= 12; i++) {
  const p = 'scenarios/moba/abilities_688g_b' + String(i).padStart(2, '0') + '.osd';
  const cur = gh('api "repos/' + REPO + '/contents/' + p + '?ref=' + head + '" --jq .sha');
  if (!cur.startsWith(pre[i])) throw new Error('origin moved for ' + p + ' ' + cur.slice(0, 10));
  const content = fs.readFileSync('/workspace/oscraft/' + p, 'utf8');
  const sha = gh('api --method POST repos/' + REPO + '/git/blobs --input - --jq .sha', JSON.stringify({ content, encoding: 'utf-8' }));
  tree.push({ path: p, mode: '100644', type: 'blob', sha });
}
const treeSha = gh('api --method POST repos/' + REPO + '/git/trees --input - --jq .sha', JSON.stringify({ base_tree: baseTree, tree }));
const message = 'Record the 688g effect-mapping outcome in each batch header: 0 of 240 rows mapped, all UNVERIFIED\n\nRow: content-688g-ability-effect-mapping\nSession: drain-688g-effects-1';
const commit = gh('api --method POST repos/' + REPO + '/git/commits --input - --jq .sha', JSON.stringify({ message, tree: treeSha, parents: [head] }));
const ref = gh('api --method PATCH repos/' + REPO + '/git/refs/heads/main --input - --jq .object.sha', JSON.stringify({ sha: commit, force: false }));
return { login, head, commit, ref, secrets: 'PASS' };
