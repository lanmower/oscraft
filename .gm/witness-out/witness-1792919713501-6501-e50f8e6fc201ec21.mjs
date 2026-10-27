import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const cp=require('child_process'),fs=require('fs'),path=require('path'),os=require('os');const R='/workspace/oscraft';const sh=(c)=>cp.execSync(c,{cwd:R,encoding:'utf8',maxBuffer:1<<26,shell:'/bin/bash'});const ok=(c,m)=>{if(!c){console.log('RESULT: FAIL '+m);process.exit(1)}};const done=(m)=>console.log('RESULT: PASS '+m);
const RUN_PAGES='40179272563',RUN_CI='40179272561';
const jobs=(run)=>JSON.parse(sh('gh run view '+run+' --repo lanmower/oscraft --json jobs')).jobs;
const serve=(args)=>new Promise((res)=>{const p=cp.spawn('node',[R+'/scripts/dev/serve.mjs',...args],{cwd:R});let b='';p.stdout.on('data',d=>{b+=d;const m=b.match(/http:\/\/localhost:(\d+)\/js/);if(m)res({p,port:m[1]})});});
const get=async(u,m='GET')=>{const r=await fetch(u,{method:m});return r};
(async()=>{try{const log=sh('gh run view 40179272563 --repo lanmower/oscraft --log');const m=log.match(/first-draw load=\d+ ms=\d+ transferred-bytes=\d+/g)||[];ok(m.length>=3,'no transferred-bytes lines');const src=fs.readFileSync(R+'/scripts/perf/first-draw.mjs','utf8');ok(/content-encoding": "gzip"/.test(src)&&/siteDir \+ path\.sep/.test(src),'gzip or sep absent');done(m.join(' | '));}catch(e){console.log('RESULT: FAIL '+e.message);process.exit(1)}})()