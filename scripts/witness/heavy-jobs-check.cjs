const { parseTop, parsePs, classify, CPU_LIMIT } = require('./lib/heavy-jobs.cjs');

const topFixture = `top - 00:00:01 up 1 day
Tasks: 3 total
  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
  101 abc       20   0  1000  2000   100 S   5.0   0.1   0:00.1 bash

top - 00:00:03 up 1 day
  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
  201 abc       20   0  1000  2000   100 R  100.0   0.1   0:00.1 gm-core
  202 abc       20   0  1000  2000   100 R    3.0   0.1   0:00.1 cc1plus
  203 abc       20   0  1000  2000   100 R    2.0   0.1   0:00.1 chromium
  204 abc       20   0  1000  2000   100 R   25.0   0.1   0:00.1 sleep
  206 abc       20   0  1000  2000   100 R   20.0   0.1   0:00.1 sleep
  `;
const psFixture = `  201 900000 /config/.gmnew/versions/x/gm-core worker --handshake {"protocolVersion":"2025-11-25"}
  202 1000 /usr/lib/gcc/cc1plus -quiet -O2 -o x.o
  203 1500000 /usr/lib/chromium/chromium --type=renderer --field-trial=x
  204 1000 sleep 5
  205 500000 /usr/lib/chromium/chromium --type=renderer --field-trial=y
  206 1000 sleep 6
  `;
const topRows = parseTop(topFixture);
const psRows = parsePs(psFixture);
const found = classify(psRows, topRows);
const kinds = (pid) => found.filter((item) => item.pid === pid).map((item) => item.kind);
const cases = [
    { name: 'a gm-core worker started with --handshake does not count at 100% CPU', run: () => kinds(201).length === 0 },
    { name: 'a C++ compiler at 3% CPU counts as compile', run: () => JSON.stringify(kinds(202)) === '["compile"]' },
    { name: 'a Chromium renderer over 1 GB counts at 2% CPU', run: () => JSON.stringify(kinds(203)) === '["chromium-renderer"]' },
    { name: 'a Chromium renderer under 1 GB does not count', run: () => kinds(205).length === 0 },
    { name: 'a 25% process not in the compiler list counts as hot', run: () => JSON.stringify(kinds(204)) === '["hot"]' },
    { name: 'exactly at the 20% limit does not count', run: () => kinds(206).length === 0 && CPU_LIMIT === 20 },
    { name: 'an empty process list is clear', run: () => classify([], []).length === 0 },
];
let fail = 0;
for (const testCase of cases) {
    const ok = testCase.run();
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${testCase.name}`);
    if (!ok) fail = 1;
}
console.log(fail ? 'heavy-jobs check FAILED' : 'heavy-jobs ok (7 cases)');
process.exit(fail);
