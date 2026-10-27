import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const B=S+'assets/voices/';const m=JSON.parse(await g(B+'manifest.json'));let miss=0,tot=0;for(const h of Object.values(m))for(const v of Object.values(h)){if(!v.endsWith('.ogg'))continue;tot++;if(!T.has(B+v))miss++;}console.log('RESULT:',miss===0&&tot>0?'PASS':'FAIL','files',tot,'missing',miss,'heroes',Object.keys(m).length);
})();