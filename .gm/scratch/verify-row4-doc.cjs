const fs = require('fs');
const doc = fs.readFileSync('docs/CONVERGENCE_688G.md', 'utf8');
const need = ['19 of its 51 creep', 'the 58 unresolved heroes', 'removed in ~ebbafb25~'.split('~').join(String.fromCharCode(96)), '0 of their 248 compared fields differ', 'lines in ~units.osd~ carry an UNVERIFIED comment'.split('~').join(String.fromCharCode(96))];
for (const s of need) console.log('present', doc.split(s).length - 1, JSON.stringify(s.slice(0, 40)));
console.log('stale 77-row text present', doc.split('(77 rows)').length - 1, '| 61 unresolved heroes present', doc.split('the 61 unresolved heroes').length - 1);
