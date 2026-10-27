import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const fs = require('fs'); const os = require('os'); const path = require('path'); const { spawnSync } = require('child_process');
(async () => {
  const sha = 'd36cb5d063c226f3c8b9d67d1436cbb329395a60';
  const got = spawnSync('gh', ['api', 'repos/lanmower/oscraft/contents/js/client/play.js?ref=' + sha, '--jq', '.content'], { encoding: 'utf8' });
  const src = Buffer.from(String(got.stdout).replace(/\s/g, ''), 'base64').toString('utf8');
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'play-check-'));
  const file = path.join(dir, 'play.mjs');
  fs.writeFileSync(file, src);
  const syntax = spawnSync(process.execPath, ['--check', file], { encoding: 'utf8' });
  fs.rmSync(dir, { recursive: true, force: true });
  const s1 = src.indexOf('let deferredUntilDrawn = null;');
  const r1 = src.indexOf('function runDeferredUntilDrawn() {', s1);
  const e1 = src.indexOf('\n}\n', r1) + 3;
  const s2 = src.indexOf('const preloadedModels = new Set();');
  const f2 = src.indexOf('function preloadModel(name, retriesLeft) {', s2);
  const e2 = src.indexOf('\n}\n', f2) + 3;
  const chunk = src.slice(s1, e1) + '\n' + src.slice(s2, e2);
  const calls = {}; const events = []; const warns = []; let ran = 0;
  const plan = { A: ['ok'], B: ['fail', 'ok'], C: ['fail', 'fail'], D: ['ok'] };
  function parsedModel(name) {
    calls[name] = (calls[name] || 0) + 1;
    const steps = plan[name] || ['ok'];
    const step = steps[Math.min(calls[name] - 1, steps.length - 1)];
    return step === 'ok' ? Promise.resolve({ name }) : Promise.reject(new Error('models/' + name + '.glb HTTP 404'));
  }
  class CE { constructor(type, init) { this.type = type; this.detail = init && init.detail; } }
  const win = { dispatchEvent(ev) { events.push(ev.type + ':' + JSON.stringify(ev.detail)); return true; } };
  const con = { warn(m) { warns.push(String(m)); } };
  const api = new Function('parsedModel', 'window', 'CustomEvent', 'console', chunk + '\nreturn { preloadPickedModels, runDeferredUntilDrawn, deferUntilDrawn, preloadedModels, awaitingFirstFrame, state: () => firstFrameUp };')(parsedModel, win, CE, con);
  const tick = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
  api.deferUntilDrawn(() => { ran += 1; });
  api.preloadPickedModels([{ model: 'A', ready: false }, { model: 'A', ready: true }, { model: '', ready: false }, { model: 'B' }, { model: 'C' }]);
  await tick();
  const pre = { calls: JSON.stringify(calls), events: events.length, awaiting: api.awaitingFirstFrame.length };
  api.runDeferredUntilDrawn(); api.runDeferredUntilDrawn();
  await tick();
  const mid = { calls: JSON.stringify(calls), events: events.slice(), warns: warns.length, ran };
  api.preloadPickedModels([{ model: 'D', ready: false }, { model: 'A' }]);
  await tick();
  const ok = pre.calls === '{}' && pre.events === 0 && pre.awaiting === 3 && mid.events.length === 2 && mid.warns === 1 && mid.ran === 1 && calls.A === 1 && calls.B === 2 && calls.C === 2 && calls.D === 1 && events.length === 3;
  console.log(JSON.stringify({ sha, fetch_status: got.status, bytes: src.length, node: process.version, syntax_status: syntax.status, pre, mid, events, warns, calls, RESULT: ok ? 'PASS' : 'FAIL' }));
})();
