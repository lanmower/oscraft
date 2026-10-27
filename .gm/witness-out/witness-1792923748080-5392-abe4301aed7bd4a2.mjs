import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');

const fs=require('fs'), path=require('path'), {execFileSync}=require('child_process');
const R='/workspace/oscraft', S=path.join(R,'.gm/drain-ci-policy-12');
const raw=(args,env)=>execFileSync('git',['-C',R].concat(args),{encoding:'utf8',env:Object.assign({},process.env,env||{}),maxBuffer:64*1024*1024});
const git=(args,env)=>raw(args,env).trim();
const base=git(['rev-parse','origin/main']);
const baseCi=raw(['show',base+':.github/workflows/ci.yml']);
const newCi=fs.readFileSync(path.join(R,'.github/workflows/ci.yml'),'utf8');
if (!newCi.startsWith(baseCi.replace(/\n+$/,'\n'))) throw new Error('ci.yml on origin is not the base of this change');
let smokeAbsent=true; try { raw(['cat-file','-e',base+':scripts/perf/page-smoke.mjs']); smokeAbsent=false; } catch(e) {}
if (!smokeAbsent) throw new Error('page-smoke already on origin');
const paths=['scripts/perf/page-smoke.mjs','.github/workflows/ci.yml'];
const blobs=paths.map(p=>git(['hash-object','-w','--',p]));
const idx=path.join(S,'tmp-index'); try{fs.unlinkSync(idx);}catch(e){}
const env={GIT_INDEX_FILE:idx};
git(['read-tree',base],env);
paths.forEach((p,i)=>git(['update-index','--add','--cacheinfo','100644,'+blobs[i]+','+p],env));
const tree=git(['write-tree'],env);
const msg=['Add the built page smoke to push CI','','ci.yml gains browser-smoke: it builds the site from the Pages cache keys (restoring them when warm), lays it out,','and runs scripts/perf/page-smoke.mjs headless with the Playwright cache the first-draw gate uses. The smoke checks that','the lobby mounts, that the page raises no uncaught or console error (external relay failures excluded), and that every','hero icon the lobby requests loads.','','Row: ci-browser-probe-steps-absent-blocker-drain-g-1',''].join('\n');
fs.writeFileSync(path.join(S,'commit-msg2.txt'),msg);
const secrets=execFileSync('node',['scripts/check-staged-secrets.mjs'],{cwd:R,encoding:'utf8'});
const commit=git(['commit-tree',tree,'-p',base,'-F',path.join(S,'commit-msg2.txt')]);
fs.unlinkSync(idx);
console.log(JSON.stringify({base,tree,commit,secretsPass:/RESULT: PASS/.test(secrets),changed:git(['diff','--stat',base,commit])}));
