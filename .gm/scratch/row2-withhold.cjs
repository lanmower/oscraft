const fs = require('fs');
const p = process.argv[2];
const src = fs.readFileSync(p, 'utf8');
const lines = src.split('\n');
const hdr = lines[0].split('\t');
const K = hdr.indexOf('shipped_kind'), R = hdr.indexOf('reason'), T = hdr.indexOf('tree_rule');
const SUFFIX = ' -- kind withheld: 6.88g gives no kind for this code, so its placements are dropped';
const shipped = l => { const c = l.split('\t'); return c[T] !== 'tree' && c[K] !== '' && c[K] !== undefined; };
const before = lines.slice(1).filter(l => l && shipped(l));
const withheld = [];
const out = [lines[0]];
for (const l of lines.slice(1)) {
  if (!l) { out.push(l); continue; }
  const c = l.split('\t');
  if (shipped(l) && !c[R].startsWith('6.88g mapped this code to ')) { withheld.push(c[0] + ' ' + c[1] + ' ' + c[K]); c[K] = ''; c[R] = c[R] + SUFFIX; }
  out.push(c.join('\t'));
}
const after = out.slice(1).filter(l => l && shipped(l));
const kinds = a => new Set(a.map(l => l.split('\t')[K])).size;
const pl = a => a.reduce((n, l) => n + Number(l.split('\t')[1] || (l.split(' ')[1] || 0)), 0);
console.log('before: rows with a kind', before.length, 'distinct kinds', kinds(before), 'placements', pl(before));
console.log('withheld rows', withheld.length, 'placements', withheld.reduce((n, s) => n + Number(s.split(' ')[1]), 0), 'first', withheld.slice(0, 6).join(' | '));
console.log('after: rows with a kind', after.length, 'distinct kinds', kinds(after), 'placements', pl(after));
if (process.argv[3] === '--write') { fs.writeFileSync(p, out.join('\n')); console.log('written'); }
