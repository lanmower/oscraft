import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const r=(await g(S+'assets/icons/ABILITY-ICONS.tsv')).split('\n').filter(l=>l&&l[0]!='#').slice(1).map(l=>l.split('\t'));const bad=r.filter(x=>x[2]!=='GAP'&&!T.has(S+'assets/icons/'+x[2]));console.log('RESULT:',r.length>300&&bad.length===0?'PASS':'FAIL','ids',r.length,'gap',r.filter(x=>x[2]==='GAP').length,'badpath',bad.length);
})();