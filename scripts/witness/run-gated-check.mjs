import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const runner = new URL("./run-gated.sh", import.meta.url).pathname;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "gate-run-"));
const bin = path.join(dir, "bin");
fs.mkdirSync(bin);
const stub = (name, body) => fs.writeFileSync(path.join(bin, name), "#!/bin/sh\n" + body, { mode: 0o755 });
stub("vmstat", [
  'echo "procs -----------memory---------- ---swap-- -----io---- -system-- -------cpu-------"',
  'echo " r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st gu"',
  'echo " 3  0 3695972 3423628   2844 4986764 $SI $SO 47940 23481 56181 18 32 8 55 5 0 0"',
  ''].join("\n"));
stub("top", [
  'frame() {',
  '  echo "top - 22:10:00 up 1 day,  3:00,  1 user,  load average: 2.00, 1.80, 1.60"',
  '  echo "  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND"',
  '  [ "$HANDSHAKE" = 1 ] && echo "1002562 user      20   0  760892 760892  1000 S 300.0   1.0  66:58.00 gm-core"',
  '  [ "$HEAVY" = 1 ] && echo "1151565 user      20   0  388808 388808  1000 S $CPU   0.5   0:56.00 chromium"',
  '  echo "  999 user      20   0    1000   1000   500 S   0.0   0.0   0:00.10 sleep"',
  '}',
  'frame',
  'frame',
  ''].join("\n"));
stub("ps", [
  'case "$*" in',
  '  *1002562*) echo "/config/gm-core worker --handshake {}" ;;',
  '  *1151565*) echo "chromium --type=gpu-process" ;;',
  '  *) echo "sleep" ;;',
  'esac',
  ''].join("\n"));

const run = (si, so, heavy = 0, handshake = 0, cpu = "475.0") => {
  const marker = path.join(dir, "ran-" + si + "-" + so + "-" + heavy + "-" + handshake + "-" + cpu);
  const r = spawnSync("bash", [runner, "touch", marker], { env: { ...process.env, PATH: bin + ":" + process.env.PATH, SI: String(si), SO: String(so), HEAVY: String(heavy), HANDSHAKE: String(handshake), CPU: cpu }, encoding: "utf8" });
  return { rc: r.status, ran: fs.existsSync(marker), out: r.stdout };
};

const closed = run(2032, 0);
expect(closed.rc === 3 && !closed.ran, "a closed reading (si 2032) must stop before the witness: rc " + closed.rc + ", ran " + closed.ran);
expect(closed.out.includes("GATE CLOSED"), "a closed reading names the stop");
const closedOut = run(21088, 0);
expect(closedOut.rc === 3 && !closedOut.ran, "a closed reading (so is the bound too) must stop before the witness");
const open = run(64, 0);
expect(open.rc === 0 && open.ran, "an open reading runs the witness: rc " + open.rc + ", ran " + open.ran);
expect(/wall=\d+\.\d\ds rc=0/.test(open.out), "an open run prints its wall time");
const busy = run(64, 0, 1, 0);
expect(busy.rc === 4 && !busy.ran, "a heavy process (chromium 475%) must stop before the witness: rc " + busy.rc + ", ran " + busy.ran);
expect(busy.out.includes("HEAVY JOB RUNNING") && busy.out.includes("1151565") && busy.out.includes("chromium"), "a heavy stop names the process by pid and command");
const low = run(64, 0, 1, 0, "10.0");
expect(low.rc === 0 && low.ran, "a process at 10% CPU does not block: rc " + low.rc + ", ran " + low.ran);
const warm = run(64, 0, 1, 0, "25.0");
expect(warm.rc === 4 && !warm.ran, "a non-handshake process at 25% CPU blocks: rc " + warm.rc + ", ran " + warm.ran);
const relay = run(64, 0, 0, 1);
expect(relay.rc === 0 && relay.ran, "a gm-core handshake relay worker at 300% does not block (full ps args, not the top name): rc " + relay.rc + ", ran " + relay.ran);

fs.rmSync(dir, { recursive: true, force: true });
console.log("run-gated ok (9 cases)");
