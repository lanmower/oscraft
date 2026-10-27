const fs = require('fs');
const lines = fs.readFileSync(process.argv[2], 'utf8').split('\n').slice(1).filter(Boolean);
const w = lines.filter(l => /kind withheld/.test(l.split('\t')[5] || ''));
const blankOK = w.filter(l => l.split('\t')[4] === '').length;
console.log('rows with kind withheld reason', w.length, 'with blank kind', blankOK, 'of which placements', w.reduce((n, l) => n + Number(l.split('\t')[1]), 0));
console.log('reasons intact (6.88g mapped rows still carry kinds):', lines.filter(l => /^6\.88g mapped/.test((l.split('\t')[5] || '')) && l.split('\t')[4] !== '').length);
