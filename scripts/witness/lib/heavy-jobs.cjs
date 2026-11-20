const COMPILERS = new Set(['cc1plus', 'cc1', 'rustc', 'cargo', 'zig', 'cmake', 'ninja', 'collect2', 'clang', 'lld', 'mold', 'sccache']);
const CPU_LIMIT = 20;
const RENDERER_RSS_KIB = 1048576;

function parseTop(text) {
    const frames = [];
    for (const line of text.split('\n')) {
        if (/^\s*PID\b/.test(line)) {
            frames.push([]);
            continue;
        }
        const fields = line.trim().split(/\s+/);
        if (frames.length === 0 || fields.length < 9 || !/^\d+$/.test(fields[0])) continue;
        frames[frames.length - 1].push({ pid: Number(fields[0]), cpu: Number(fields[8]) });
    }
    return frames.length ? frames[frames.length - 1] : [];
}

function parsePs(text) {
    return text.split('\n').map((line) => line.trim()).filter(Boolean).map((line) => {
        const [pid, rss, ...rest] = line.split(/\s+/);
        return { pid: Number(pid), rss: Number(rss), args: rest.join(' ') };
    });
}

function classify(psRows, topRows) {
    const cpu = new Map(topRows.map((row) => [row.pid, row.cpu]));
    const found = [];
    for (const row of psRows) {
        const words = row.args.split(' ');
        const base = (words[0] || '').split('/').pop();
        if (/gm-core/.test(row.args) && /--handshake/.test(row.args)) continue;
        if (base === 'cargo' && ['test', 'check', 'nextest'].includes(words[1])) found.push({ kind: 'test-check', pid: row.pid, args: row.args });
        else if (COMPILERS.has(base)) found.push({ kind: 'compile', pid: row.pid, args: row.args });
        else if (/--type=renderer/.test(row.args) && /chrom/.test(row.args) && row.rss > RENDERER_RSS_KIB) found.push({ kind: 'chromium-renderer', pid: row.pid, args: row.args });
        if (cpu.has(row.pid) && cpu.get(row.pid) > CPU_LIMIT && !found.some((item) => item.pid === row.pid)) found.push({ kind: 'hot', pid: row.pid, cpu: cpu.get(row.pid), args: row.args });
    }
    return found;
}

module.exports = { parseTop, parsePs, classify, CPU_LIMIT };
