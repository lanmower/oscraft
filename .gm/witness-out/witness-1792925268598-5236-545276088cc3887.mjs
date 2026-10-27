import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const {execSync}=require('child_process');
const S='a090c61de494fbf936e83d4963f317cf086bbc65';
const api=(p)=>JSON.parse(execSync('gh api "'+p+'"',{encoding:'utf8',maxBuffer:64*1024*1024}));
const file=(p)=>Buffer.from(api('repos/lanmower/oscraft/contents/'+p+'?ref='+S).content,'base64').toString('utf8');
const ci=file('.github/workflows/ci.yml'), tests=file('.github/workflows/tests.yml'), pages=file('.github/workflows/pages.yml');
const jobsOf=(t)=>t.split('\n').filter(l=>/^  [a-z][a-z-]*:$/.test(l)).map(l=>l.trim().slice(0,-1)).filter(n=>n!=='push');
const triggers=(t)=>{const i=t.indexOf('\non:'); const j=t.indexOf('\npermissions:'); return t.slice(i,j).split('\n').filter(l=>/^  [a-z_]+:/.test(l)).map(l=>l.trim());};
const testCommands=/cargo nextest|cargo test|node test\.js|parity_hash|wasm-hash\.mjs/;
const checks = {
  ciJobs: jobsOf(ci), ciTriggers: triggers(ci),
  ciRunsTestCommands: testCommands.test(ci),
  testsTriggers: triggers(tests), testsJobs: jobsOf(tests),
  testsHasPush: /\n  push:/.test(tests),
  pagesGateWireRemaining: pages.includes('gate-wire'), pagesDeployNeeds: (pages.match(/needs: \[[^\]]*\]/g)||[]).join(' ')
};
const runs=api('repos/lanmower/oscraft/actions/runs?head_sha='+S+'&per_page=20').workflow_runs;
const wall=(r)=>Math.round((Date.parse(r.updated_at)-Date.parse(r.run_started_at))/1000);
const timing={}; for (const r of runs) timing[r.name]={runId:r.id, conclusion:r.conclusion, wallSec: wall(r)};
const pass = checks.ciJobs.join()==='static,browser-smoke' && !checks.ciRunsTestCommands && checks.testsTriggers.join()==='workflow_dispatch:' && checks.testsJobs.join()==='rust,wasm,parity,maps,wire' && !checks.testsHasPush && !checks.pagesGateWireRemaining && checks.pagesDeployNeeds.includes('gate-static') && !checks.pagesDeployNeeds.includes('gate-wire') && timing.ci && timing.ci.conclusion==='success' && timing.pages && timing.pages.conclusion==='success';
console.log(JSON.stringify({sha:S, checks, timing, RESULT: pass ? 'PASS' : 'FAIL'}));
