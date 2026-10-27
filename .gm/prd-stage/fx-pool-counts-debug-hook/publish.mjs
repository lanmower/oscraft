import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
const dir = '/workspace/oscraft/.gm/prd-stage/fx-pool-counts-debug-hook';
const repo = 'lanmower/oscraft';
const base = 'd36cb5d063c226f3c8b9d67d1436cbb329395a60';
const gh = (args, input) => execFileSync('gh', ['api', ...args], { input, encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
if (head !== base) throw new Error('origin main moved: ' + head + ' != ' + base);
const baseTree = JSON.parse(gh([`repos/${repo}/git/commits/${base}`])).tree.sha;
const blob = (content) => JSON.parse(gh(['-X', 'POST', `repos/${repo}/git/blobs`, '--input', '-'], JSON.stringify({ content, encoding: 'utf-8' }))).sha;
const tree = [
  { path: 'js/client/debug_api.js', mode: '100644', type: 'blob', sha: blob(fs.readFileSync(dir + '/debug_api.js', 'utf8')) },
  { path: 'js/client/play.js', mode: '100644', type: 'blob', sha: blob(fs.readFileSync(dir + '/play.js', 'utf8')) },
];
const treeSha = JSON.parse(gh(['-X', 'POST', `repos/${repo}/git/trees`, '--input', '-'], JSON.stringify({ base_tree: baseTree, tree }))).sha;
const message = 'Debug API: oscraft.debug.fx() returns the effect pool counts\n\n' +
  'fx.counts() was not reachable from window.oscraft, so pool occupancy could not be read from the page.\n' +
  'debug_api.js gains a read-only debug.fx() that returns fx.counts() (one count per effect pool kind, and costMs);\n' +
  'play.js passes the live fx instance through the existing hooks object. No change to rendering, the sim or the\n' +
  'first-draw path.\n\nRow: fx-pool-counts-debug-hook\n';
const commit = JSON.parse(gh(['-X', 'POST', `repos/${repo}/git/commits`, '--input', '-'], JSON.stringify({ message, tree: treeSha, parents: [base] }))).sha;
gh(['-X', 'PATCH', `repos/${repo}/git/refs/heads/main`, '--input', '-'], JSON.stringify({ sha: commit, force: false }));
console.log('PUBLISHED', commit, 'TREE', treeSha, 'BASE', base);
