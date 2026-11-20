#!/usr/bin/env node
const { execFileSync } = require('node:child_process');
const { parseTop, parsePs, classify } = require('./lib/heavy-jobs.cjs');

const top = execFileSync('top', ['-b', '-n', '2', '-d', '1', '-w', '200'], { encoding: 'utf8' });
const ps = execFileSync('ps', ['-eo', 'pid=,rss=,args='], { encoding: 'utf8' });
const found = classify(parsePs(ps), parseTop(top));
if (found.length === 0) {
    console.log('HEAVY_JOBS clear');
    process.exit(0);
}
console.log(`HEAVY_JOBS busy n=${found.length}`);
for (const item of found) {
    console.log(`${item.kind} pid=${item.pid}${item.cpu === undefined ? '' : ` cpu=${item.cpu}`} ${item.args.slice(0, 160)}`);
}
process.exit(1);
