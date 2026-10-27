import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const cp=require('child_process'),fs=require('fs'),path=require('path'),os=require('os');const R='/workspace/oscraft';const sh=(c)=>cp.execSync(c,{cwd:R,encoding:'utf8',maxBuffer:1<<26,shell:'/bin/bash'});const ok=(c,m)=>{if(!c){console.log('RESULT: FAIL '+m);process.exit(1)}};const done=(m)=>console.log('RESULT: PASS '+m);
const RUN_PAGES='40179272563',RUN_CI='40179272561';
const jobs=(run)=>JSON.parse(sh('gh run view '+run+' --repo lanmower/oscraft --json jobs')).jobs;
const serve=(args)=>new Promise((res)=>{const p=cp.spawn('node',[R+'/scripts/dev/serve.mjs',...args],{cwd:R});let b='';p.stdout.on('data',d=>{b+=d;const m=b.match(/http:\/\/localhost:(\d+)\/js/);if(m)res({p,port:m[1]})});});
const get=async(u,m='GET')=>{const r=await fetch(u,{method:m});return r};
(async()=>{try{const t=fs.readFileSync(R+'/AGENTS.md','utf8');for(const k of ['layout-site.sh','stage.sh','serve.mjs','first-draw.mjs','check-modulepreload.mjs','rust-toolchain.toml','hmr.js','ceiling'])ok(t.includes(k),'missing '+k);ok(t.length<30000,'too big');done('AGENTS.md '+t.length+' bytes carries the build, CI, Pages, dev server, first-draw and cache facts');}catch(e){console.log('RESULT: FAIL '+e.message);process.exit(1)}})()