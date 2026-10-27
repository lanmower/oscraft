import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const fs=require('fs'), path=require('path'), {execFileSync}=require('child_process');
const R='/workspace/oscraft', S=path.join(R,'.gm/drain-ci-policy-12');
const raw=(args,env)=>execFileSync('git',['-C',R].concat(args),{encoding:'utf8',env:Object.assign({},process.env,env||{}),maxBuffer:64*1024*1024});
const git=(args,env)=>raw(args,env).trim();
const base=git(['rev-parse','origin/main']);
const originCi=raw(['show',base+':.github/workflows/ci.yml']);
const originPages=raw(['show',base+':.github/workflows/pages.yml']);
if (originCi!==fs.readFileSync(path.join(S,'origin-ci.yml'),'utf8') || originPages!==fs.readFileSync(path.join(S,'origin-pages.yml'),'utf8')) throw new Error('origin moved since the read; re-derive');
const paths=['.github/workflows/tests.yml','.github/workflows/ci.yml','.github/workflows/pages.yml'];
const blobs=paths.map(p=>git(['hash-object','-w','--',p]));
const idx=path.join(S,'tmp-index'); try{fs.unlinkSync(idx);}catch(e){}
const env={GIT_INDEX_FILE:idx};
git(['read-tree',base],env);
paths.forEach((p,i)=>git(['update-index','--add','--cacheinfo','100644,'+blobs[i]+','+p],env));
const tree=git(['write-tree'],env);
const msg=['Move the test jobs to an on-demand workflow','','The Rust workspace tests, the wasm32 and wasip3 compile, the native and wasm state-hash parity, the map export','determinism check and the wire codec round-trip move from ci.yml to tests.yml, which runs on workflow_dispatch only.','ci.yml keeps the static client checks; pages.yml drops its wire gate and keeps the static gate, the deploy and the','first-draw gate. Test sources stay in git.','','Row: ci-tests-on-demand-speed-decision',''].join('\n');
fs.writeFileSync(path.join(S,'commit-msg.txt'),msg);
const secrets=execFileSync('node',['scripts/check-staged-secrets.mjs'],{cwd:R,encoding:'utf8'});
const commit=git(['commit-tree',tree,'-p',base,'-F',path.join(S,'commit-msg.txt')]);
fs.unlinkSync(idx);
const changed=git(['diff','--stat',base,commit]);
console.log(JSON.stringify({base,tree,blobs,commit,secretsPass:/RESULT: PASS/.test(secrets),changed}));
