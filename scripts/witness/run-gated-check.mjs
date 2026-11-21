import { expect } from "../../js/client/check-expect.mjs";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const runner = new URL("./run-gated.sh", import.meta.url).pathname;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "gate-run-"));
const bin = path.join(dir, "bin");
fs.mkdirSync(bin);
const stub = (name, body) => fs.writeFileSync(path.join(bin, name), "#!/bin/sh\n" + body + "\n", { mode: 0o755 });
stub("vmstat", [
  'echo "procs -----------memory---------- ---swap-- -----io---- -system-- -------cpu-------"',
  'echo " r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st gu"',
  'echo " 3  0 3695972 3423628   2844 4986764 $SI $SO 47940 23481 56181 18 32 8 55 5 0 0"'].join("\n"));
stub("top", 'cat "$TOPFIX"');
stub("ps", 'cat "$PSFIX"');

const topFrame = (rows) => [
  "top - 00:00:01 up 1 day,  3:00,  1 user,  load average: 2.00, 1.80, 1.60",
  "  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND",
  ...rows.map(([pid, cpu, name]) => pid + " user      20   0  760892  760892   1000 S " + cpu.toFixed(1) + "   1.0  66:58.00 " + name)].join("\n");
const topFixture = (rows) => topFrame(rows) + "\n" + topFrame(rows) + "\n";
const psFixture = (rows) => rows.map(([pid, rss, args]) => pid + " " + rss + " " + args).join("\n") + "\n";

const run = (name, { si = 64, so = 0, top = [], ps = [] }) => {
  const id = name.replace(/\W+/g, "-");
  const topFile = path.join(dir, id + ".top");
  const psFile = path.join(dir, id + ".ps");
  const marker = path.join(dir, "ran-" + id);
  fs.writeFileSync(topFile, topFixture(top));
  fs.writeFileSync(psFile, psFixture(ps));
  const env = { ...process.env, PATH: bin + ":" + process.env.PATH, SI: String(si), SO: String(so), TOPFIX: topFile, PSFIX: psFile };
  const r = spawnSync("bash", [runner, "touch", marker], { env, encoding: "utf8" });
  return { rc: r.status, ran: fs.existsSync(marker), out: r.stdout };
};

const idle = [[999, 0.0, "sleep"]];
const idlePs = [[999, 1000, "sleep 5"]];

const closed = run("closed-si", { si: 2032, top: idle, ps: idlePs });
expect(closed.rc === 3 && !closed.ran && closed.out.includes("GATE CLOSED"), "a closed reading (si 2032) stops before the witness: rc " + closed.rc);
const closedSo = run("closed-so", { so: 21088, top: idle, ps: idlePs });
expect(closedSo.rc === 3 && !closedSo.ran, "a closed reading (so 21088) stops before the witness: rc " + closedSo.rc);
const open = run("open", { top: idle, ps: idlePs });
expect(open.rc === 0 && open.ran && /wall=\d+\.\d\ds rc=0/.test(open.out), "an open, clear reading runs the witness and prints its wall time: rc " + open.rc);
const relay = run("relay", { top: [[1002562, 300.0, "gm-core"]], ps: [[1002562, 900000, "/config/gm-core worker --handshake {}"]] });
expect(relay.rc === 0 && relay.ran, "a gm-core handshake relay worker at 300% does not block: rc " + relay.rc);
const gpu = run("gpu", { top: [[1151565, 475.0, "chromium"]], ps: [[1151565, 388808, "/usr/lib/chromium/chromium --type=gpu-process"]] });
expect(gpu.rc === 4 && !gpu.ran && gpu.out.includes("1151565") && gpu.out.includes("HEAVY JOB RUNNING"), "a non-handshake chromium gpu process at 475% blocks, named by pid: rc " + gpu.rc);
const low = run("sleep-10", { top: [[998, 10.0, "sleep"]], ps: [[998, 1000, "sleep 9"]] });
expect(low.rc === 0 && low.ran, "a process at 10% CPU does not block: rc " + low.rc);
const warm = run("sleep-25", { top: [[998, 25.0, "sleep"]], ps: [[998, 1000, "sleep 9"]] });
expect(warm.rc === 4 && !warm.ran, "a non-compiler process at 25% CPU blocks: rc " + warm.rc);
const cc = run("cc1plus-3", { top: [[202, 3.0, "cc1plus"]], ps: [[202, 1000, "/usr/lib/gcc/cc1plus -quiet -O2 -o x.o"]] });
expect(cc.rc === 4 && !cc.ran && cc.out.includes("compile pid=202"), "a C++ compiler at 3% CPU blocks as a compile: rc " + cc.rc);
const big = run("renderer-big", { top: [[203, 2.0, "chromium"]], ps: [[203, 1500000, "/usr/lib/chromium/chromium --type=renderer --field-trial=x"]] });
expect(big.rc === 4 && !big.ran && big.out.includes("chromium-renderer pid=203"), "a Chromium renderer over 1 GB at 2% CPU blocks: rc " + big.rc);
const small = run("renderer-small", { top: [[205, 2.0, "chromium"]], ps: [[205, 500000, "/usr/lib/chromium/chromium --type=renderer --field-trial=y"]] });
expect(small.rc === 0 && small.ran, "a Chromium renderer under 1 GB at 2% CPU does not block: rc " + small.rc);

fs.rmSync(dir, { recursive: true, force: true });
console.log("run-gated ok (10 cases)");
