import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const c=(await g(S+'assets/voices/COVERAGE.tsv')).split('\n').filter(l=>l&&l[0]!='#').slice(1);const abs=c.filter(l=>l.includes('UNVERIFIED-absent')).length;console.log('RESULT:',c.length>=69?'PASS':'FAIL','rows',c.length,'absent',abs);
})();