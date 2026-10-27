import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async()=>{const SHA='a0cdea01bd99accca90e3fa5c1db7523b4577b49';const R='https://raw.githubusercontent.com/lanmower/oscraft/'+SHA+'/';const g=async p=>await (await fetch(R+p)).text();
const tr=await (await fetch('https://api.github.com/repos/lanmower/oscraft/git/trees/'+SHA+'?recursive=1')).json();const T=new Set(tr.tree.map(x=>x.path));const S='scenarios/moba/';
const s=await g(S+'assets/icons/ITEM-SOURCES.tsv');const ids=[...(await g(S+'items.osd')).matchAll(/^item\("([^"]+)"/gm)].map(m=>m[1]);const no=ids.filter(i=>!T.has(S+'assets/icons/items/'+i+'.png'));console.log('RESULT:',s.includes('PLACEHOLDER')&&no.length===0?'PASS':'FAIL','items',ids.length,'noicon',no.length);
})();