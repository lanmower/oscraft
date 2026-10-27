import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const fs=require('fs');(async()=>{const R='https://raw.githubusercontent.com/lanmower/oscraft/a0cdea01bd99accca90e3fa5c1db7523b4577b49/';const g=async p=>await (await fetch(R+p)).text();const T=new Set(fs.readFileSync('/tmp/tree-a0cdea01.txt','utf8').split('\n'));const S='scenarios/moba/';
const s=await g(S+'assets/icons/ITEM-SOURCES.tsv');const ids=[...(await g(S+'items.osd')).matchAll(/^item\("([^"]+)"/gm)].map(m=>m[1]);const no=ids.filter(i=>!T.has(S+'assets/icons/items/'+i+'.png'));console.log('RESULT:',s.includes('PLACEHOLDER')&&no.length===0?'PASS':'FAIL','items',ids.length,'noicon',no.length);
})();