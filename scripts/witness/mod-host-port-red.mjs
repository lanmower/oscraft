import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const checker = path.join(root, "scripts", "check-mod-host-port.mjs");
const realCrate = path.join(root, "rust", "crates", "mod-host");
const scratch = path.join(root, "..", "tmp", "oscraft", "port");

const copyCrate = (name, mutate) => {
  const dir = path.join(scratch, name);
  fs.rmSync(dir, { recursive: true, force: true });
  fs.mkdirSync(path.join(dir, "src"), { recursive: true });
  fs.mkdirSync(path.join(dir, "tests"), { recursive: true });
  const files = { lib: "lib.rs", piccolo: "piccolo_host.rs", runtime: "runtime.rs" };
  const text = Object.fromEntries(Object.entries(files).map(([key, file]) => [key, fs.readFileSync(path.join(realCrate, "src", file), "utf8")]));
  const tests = Object.fromEntries(fs.readdirSync(path.join(realCrate, "tests")).filter((f) => f.endsWith(".rs")).map((f) => [f, fs.readFileSync(path.join(realCrate, "tests", f), "utf8")]));
  mutate?.(text, tests);
  for (const [key, file] of Object.entries(files)) fs.writeFileSync(path.join(dir, "src", file), text[key]);
  for (const [file, body] of Object.entries(tests)) fs.writeFileSync(path.join(dir, "tests", file), body);
  return dir;
};
const run = (crate) => spawnSync(process.execPath, [checker, `--crate=${crate}`], { encoding: "utf8" });

const failures = [];
const expect = (name, ok) => {
  console.log(`${ok ? "ok" : "FAIL"} ${name}`);
  if (!ok) failures.push(name);
};

let result = run(copyCrate("real"));
expect("the real crate passes with the port", result.status === 0 && result.stdout.includes("mod-host port ok"));

result = run(copyCrate("no-impl", (text) => {
  const method = "    fn players(&self) -> Vec<usize> {\n        ModHost::players(self)\n    }\n";
  if (text.runtime.split(method).length !== 2) throw new Error("players impl anchor");
  text.runtime = text.runtime.replace(method, "");
}));
expect("a trait method with no ModHost impl fails and is named", result.status === 1 && result.stderr.includes("ModRuntime method players is not implemented"));

result = run(copyCrate("stray-call", (text, tests) => {
  const file = Object.keys(tests).find((f) => tests[f].includes("host.advance("));
  tests[file] = tests[file].replace("host.advance(", "host.feed_death(");
}));
expect("a test calling a method off the port fails and is named", result.status === 1 && result.stderr.includes("host.feed_death"));

result = run(copyCrate("script-api", (text) => {
  if (!text.piccolo.includes('"on_frame"')) throw new Error("on_frame literal");
  text.piccolo = text.piccolo.split('"on_frame"').join('"on_frame_gone"');
}));
expect("a script hook missing from the host fails and is named", result.status === 1 && result.stderr.includes("script API on_frame is not registered"));

result = run(copyCrate("no-import", (text, tests) => {
  const file = Object.keys(tests).find((f) => tests[f].includes("host.advance("));
  tests[file] = tests[file].split("ModRuntime").join("Absent");
}));
expect("a test file calling the port without importing ModRuntime fails and is named", result.status === 1 && result.stderr.includes("does not import ModRuntime"));

if (failures.length) {
  console.error(`mod-host port red tests failed: ${failures.join("; ")}`);
  process.exit(1);
}
console.log("mod-host port red tests ok (5 cases)");
