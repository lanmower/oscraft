import { execFileSync } from 'node:child_process';
const repo = 'lanmower/oscraft';
const gh = (args, input) => execFileSync('gh', ['api', ...args], { input, encoding: 'utf8', maxBuffer: 1 << 26 }).trim();
const head = gh([`repos/${repo}/git/refs/heads/main`, '--jq', '.object.sha']);
const tree = JSON.parse(gh([`repos/${repo}/git/commits/${head}`])).tree.sha;
const message = 'Record closed row abaddon-and-underlord-abilities-blank-hud\n\nThe premise (no unit row, no ability rows for abaddon or abyssal_underlord) is refuted on origin:\nunit rows and hero_abilities lists exist for both heroes, and every named ability has one definition.\nThe row was filed as resolved by the store; the record here is the refutation.\n\nRow: abaddon-and-underlord-abilities-blank-hud\nWitness: 1792930261554-5213-419a50cf0dafeaee\nStatus: stale\n';
const commit = JSON.parse(gh(['-X', 'POST', `repos/${repo}/git/commits`, '--input', '-'], JSON.stringify({ message, tree, parents: [head] }))).sha;
gh(['-X', 'PATCH', `repos/${repo}/git/refs/heads/main`, '--input', '-'], JSON.stringify({ sha: commit, force: false }));
console.log('RECORDED', commit, 'ON', head);
