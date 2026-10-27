import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const sm=await g(S+'slot-models.tsv');const um={};(await g(S+'units.osd')).split('\n').forEach(l=>{const m=l.match(/^unit\("([^"]+)".*?model="([^"]+)"/);if(m)um[m[1]]=m[2];});let bad=0,n=0;for(const l of sm.split('\n').slice(1)){const c=l.split('\t');if(/^neutral_/.test(c[0])){n++;if(c[1]!==um[c[0]]||!T.has(S+'assets/models/'+c[1]+'.glb'))bad++;}}console.log('RESULT:',!sm.includes('stand-in')&&bad===0&&n>0?'PASS':'FAIL','neutral slots',n,'mismatch',bad,'stand-in',sm.includes('stand-in'));
})();