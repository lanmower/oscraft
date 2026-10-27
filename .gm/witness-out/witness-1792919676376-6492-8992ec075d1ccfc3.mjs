import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const cp=require('child_process'),fs=require('fs'),path=require('path'),os=require('os');const R='/workspace/oscraft';const sh=(c)=>cp.execSync(c,{cwd:R,encoding:'utf8',maxBuffer:1<<26,shell:'/bin/bash'});const ok=(c,m)=>{if(!c){console.log('RESULT: FAIL '+m);process.exit(1)}};const done=(m)=>console.log('RESULT: PASS '+m);
const RUN_PAGES='40179272563',RUN_CI='40179272561';
const jobs=(run)=>JSON.parse(sh('gh run view '+run+' --repo lanmower/oscraft --json jobs')).jobs;
const serve=(args)=>new Promise((res)=>{const p=cp.spawn('node',[R+'/scripts/dev/serve.mjs',...args],{cwd:R});let b='';p.stdout.on('data',d=>{b+=d;const m=b.match(/http:\/\/localhost:(\d+)\/js/);if(m)res({p,port:m[1]})});});
const get=async(u,m='GET')=>{const r=await fetch(u,{method:m});return r};
(async()=>{try{const j=jobs(RUN_CI);const w=j.find(x=>x.name==='wasm32 and wasip3 compile');ok(w&&w.conclusion==='success','wasm job not green');const log=sh('gh run view 40179272561 --repo lanmower/oscraft --job '+w.databaseId+' --log');ok(/wasm32-unknown-unknown/.test(log)&&/Checking wasip3-guest|wasip3/.test(log),'log lacks target check');done('job '+w.databaseId+' green; web wasm32 and wasip3 checked');}catch(e){console.log('RESULT: FAIL '+e.message);process.exit(1)}})()