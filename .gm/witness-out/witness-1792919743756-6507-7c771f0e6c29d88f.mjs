import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const cp=require('child_process'),fs=require('fs');const R='/workspace/oscraft';const ok=(c,m)=>{if(!c){console.log('RESULT: FAIL '+m);process.exit(1)}};
const p=cp.spawn('node',[R+'/scripts/dev/serve.mjs','/nonexistent-site'],{cwd:R});let b='';
p.stdout.on('data',async d=>{b+=d;const m=b.match(/localhost:(\d+)\/js/);if(!m)return;p.stdout.removeAllListeners('data');const t={};
for(const [u,w] of [['/js/client/shaders/mesh.frag.glsl','text/plain'],['/scenarios/moba/assets/voices/abaddon/000.ogg','audio/ogg'],['/scenarios/moba/moba.osdt','application/octet-stream'],['/scenarios/moba/doodads-w3.tsv','text/tab-separated-values']]){const r=await fetch('http://localhost:'+m[1]+u,{method:'HEAD'});t[u]=r.status+' '+r.headers.get('content-type');if(r.status!==200||!r.headers.get('content-type').startsWith(w)){console.log('RESULT: FAIL '+JSON.stringify(t));p.kill();process.exit(1)}}
p.kill();ok(/from "\.\.\/dev\/mime\.mjs"/.test(fs.readFileSync(R+'/scripts/perf/first-draw.mjs','utf8')),'first-draw not on mime module');console.log('RESULT: PASS '+JSON.stringify(t))});