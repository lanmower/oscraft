import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const fs=require('fs'), path=require('path'), {execFileSync}=require('child_process');
const R='/workspace/oscraft', S=path.join(R,'.gm/drain-ci-policy-12');
const raw=(args,env)=>execFileSync('git',['-C',R].concat(args),{encoding:'utf8',env:Object.assign({},process.env,env||{}),maxBuffer:64*1024*1024});
const git=(args,env)=>raw(args,env).trim();
const base=git(['rev-parse','origin/main']);
const blob=git(['hash-object','-w','--',path.join(S,'page-smoke.mjs')]);
const idx=path.join(S,'tmp-index'); try{fs.unlinkSync(idx);}catch(e){}
const env={GIT_INDEX_FILE:idx};
git(['read-tree',base],env);
git(['update-index','--add','--cacheinfo','100644,'+blob+',scripts/perf/page-smoke.mjs'],env);
const tree=git(['write-tree'],env);
const msg=['Serve the browser smoke under a name that is not localhost','','The smoke served the site on 127.0.0.1, so hmr.js ran as it does on localhost and asked the server for','/__hmr, which a static site does not have. It now maps a non-local name to the server, as the live page is','served, so the dev reload stays off and the smoke sees what a player sees.','','Row: ci-browser-probe-steps-absent-blocker-drain-g-1',''].join('\n');
fs.writeFileSync(path.join(S,'commit-msg4.txt'),msg);
const secrets=execFileSync('node',['scripts/check-staged-secrets.mjs'],{cwd:R,encoding:'utf8'});
const commit=git(['commit-tree',tree,'-p',base,'-F',path.join(S,'commit-msg4.txt')]);
fs.unlinkSync(idx);
console.log(JSON.stringify({base,commit,secretsPass:/RESULT: PASS/.test(secrets),changed:git(['diff','--stat',base,commit])}));
