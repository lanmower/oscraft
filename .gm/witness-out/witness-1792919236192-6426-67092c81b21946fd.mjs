import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const fs=require('fs');(async()=>{const R='https://raw.githubusercontent.com/lanmower/oscraft/a0cdea01bd99accca90e3fa5c1db7523b4577b49/';const g=async p=>await (await fetch(R+p)).text();const T=new Set(fs.readFileSync('/tmp/tree-a0cdea01.txt','utf8').split('\n'));const S='scenarios/moba/';
const r=(await g(S+'assets/icons/ABILITY-ICONS.tsv')).split('\n').filter(l=>l&&l[0]!='#').slice(1).map(l=>l.split('\t'));const bad=r.filter(x=>x[2]!=='GAP'&&!T.has(S+'assets/icons/'+x[2]));console.log('RESULT:',r.length>300&&bad.length===0?'PASS':'FAIL','ids',r.length,'gap',r.filter(x=>x[2]==='GAP').length,'badpath',bad.length);
})();