import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const fs=require('fs'), path=require('path'), {execFileSync}=require('child_process');
const R='/workspace/oscraft', S=path.join(R,'.gm/drain-ci-policy-12');
const raw=(args,env)=>execFileSync('git',['-C',R].concat(args),{encoding:'utf8',env:Object.assign({},process.env,env||{}),maxBuffer:64*1024*1024});
const git=(args,env)=>raw(args,env).trim();
const base=git(['rev-parse','origin/main']);
const basePath=git(['rev-parse',base+':scripts/perf/page-smoke.mjs']);
const blob=git(['hash-object','-w','--','scripts/perf/page-smoke.mjs']);
if (blob===basePath) throw new Error('no change to publish');
const idx=path.join(S,'tmp-index'); try{fs.unlinkSync(idx);}catch(e){}
const env={GIT_INDEX_FILE:idx};
git(['read-tree',base],env);
git(['update-index','--add','--cacheinfo','100644,'+blob+',scripts/perf/page-smoke.mjs'],env);
const tree=git(['write-tree'],env);
const msg=['Smoke every local response and wait for the hero icons','','The browser smoke names the URL of every local 4xx or 5xx, so a broken asset shows in the log instead of as a','bare console line, and it waits for the lobby hero icons whatever else it found. favicon.ico and external','relay or STUN failures are not page assets and are not counted.','','Row: ci-browser-probe-steps-absent-blocker-drain-g-1',''].join('\n');
fs.writeFileSync(path.join(S,'commit-msg3.txt'),msg);
const secrets=execFileSync('node',['scripts/check-staged-secrets.mjs'],{cwd:R,encoding:'utf8'});
const commit=git(['commit-tree',tree,'-p',base,'-F',path.join(S,'commit-msg3.txt')]);
fs.unlinkSync(idx);
console.log(JSON.stringify({base,commit,secretsPass:/RESULT: PASS/.test(secrets),changed:git(['diff','--stat',base,commit])}));
