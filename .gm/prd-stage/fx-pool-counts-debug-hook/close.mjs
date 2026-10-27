import { execFileSync } from 'node:child_process';
const repo = 'lanmower/oscraft';
const gh = (args, input) => execFileSync('gh', ['api', ...args], { input, encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const tree = JSON.parse(gh([`repos/${repo}/git/commits/${head}`])).tree.sha;
const message = 'Record closed row fx-pool-counts-debug-hook\n\nThe debug API exposes live effect pool counts as oscraft.debug.fx(); the change is in 0533566b.\n\nRow: fx-pool-counts-debug-hook\nWitness: 1792928899793-5852-3cc69f31a9aae45a\nStatus: resolved\n';
const commit = JSON.parse(gh(['-X', 'POST', `repos/${repo}/git/commits`, '--input', '-'], JSON.stringify({ message, tree, parents: [head] }))).sha;
gh(['-X', 'PATCH', `repos/${repo}/git/refs/heads/main`, '--input', '-'], JSON.stringify({ sha: commit, force: false }));
console.log('RECORDED', commit, 'ON', head, 'TREE', tree);
