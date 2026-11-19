import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const runner = new URL("./run-gated.sh", import.meta.url).pathname;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "gate-run-"));
const bin = path.join(dir, "bin");
fs.mkdirSync(bin);
fs.writeFileSync(path.join(bin, "vmstat"), `#!/bin/sh
echo "procs -----------memory---------- ---swap-- -----io---- -system-- -------cpu-------"
echo " r  b   swpd   free   buff  cache   si   so    bi    bo   in   cs us sy id wa st gu"
echo " 3  0 3695972 3423628   2844 4986764 $SI $SO 47940 23481 56181 18 32 8 55 5 0 0"
`, { mode: 0o755 });

const run = (si, so) => {
  const marker = path.join(dir, `ran-${si}-${so}`);
  const r = spawnSync("bash", [runner, "touch", marker], { env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, SI: String(si), SO: String(so) }, encoding: "utf8" });
  return { rc: r.status, ran: fs.existsSync(marker), out: r.stdout };
};

const closed = run(2032, 0);
expect(closed.rc === 3 && !closed.ran, `a closed reading (si 2032) must stop before the witness: rc ${closed.rc}, ran ${closed.ran}`);
expect(closed.out.includes("GATE CLOSED"), "a closed reading names the stop");
const closedOut = run(21088, 0);
expect(closedOut.rc === 3 && !closedOut.ran, "a closed reading (so is the bound too) must stop before the witness");
const open = run(64, 0);
expect(open.rc === 0 && open.ran, `an open reading runs the witness: rc ${open.rc}, ran ${open.ran}`);
expect(/wall=\d+\.\d\ds rc=0/.test(open.out), "an open run prints its wall time");

fs.rmSync(dir, { recursive: true, force: true });
console.log("run-gated ok (4 cases)");
