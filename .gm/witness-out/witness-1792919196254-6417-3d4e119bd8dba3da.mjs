import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const u=await g(S+'units.osd');const sm=await g(S+'slot-models.tsv');const k=['bounty-rune','observer-ward','sentry-ward'].map(x=>u.includes('content-model-request-'+x)&&sm.includes('content-model-request-'+x));const co=sm.split('\n').find(l=>l.startsWith('courier\t'));console.log('RESULT:',k.every(Boolean)&&co.includes('courier-skinned')&&T.has(S+'assets/models/courier-skinned.glb')?'PASS':'FAIL',k.join(),co.split('\t')[1]);
})();