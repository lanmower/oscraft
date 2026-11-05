import { execFileSync } from "node:child_process";

const PATTERNS = [
  ["github token", /\b(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})\b/],
  ["aws access key", /\bAKIA[0-9A-Z]{16}\b/],
  ["private key block", /-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ["bearer token", /\bBearer\s+[A-Za-z0-9._~+/-]{20,}=*/],
  ["generic secret", /\b(api[_-]?key|secret|token|password)\s*[:=]\s*["'][^"'\s]{16,}["']/i],
];

const SAMPLES = [
  ["github token", "const key = '" + "ghp_" + "a".repeat(36) + "';"],
  ["aws access key", "const key = '" + "AKIA" + "A".repeat(16) + "';"],
  ["private key block", "-----BEGIN " + "RSA " + "PRIVATE KEY-----"],
  ["bearer token", "Authorization: " + "Bearer " + "a".repeat(24)],
  ["generic secret", "const password = '" + "b".repeat(20) + "';"],
];

export const matchLine = (line) => PATTERNS.filter(([name, re]) => re.test(line)).map(([name]) => name);

export const selfTest = () => {
  const failures = [];
  for (const [name, sample] of SAMPLES) if (!matchLine(sample).includes(name)) failures.push(name);
  if (matchLine("const hero = 'antimage';").length) failures.push("benign line");
  return failures;
};

const stagedPaths = () => execFileSync("git", ["diff", "--cached", "--name-only", "--diff-filter=ACM"], { encoding: "utf8" }).split("\n").filter(Boolean);

const addedLines = (path) => execFileSync("git", ["diff", "--cached", "--unified=0", "--", path], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 }).split("\n").filter((l) => l.startsWith("+") && !l.startsWith("+++")).map((l) => l.slice(1));

if (process.argv.includes("--self-test")) {
  const failures = selfTest();
  if (failures.length) { console.error("secret patterns failed: " + failures.join(", ")); process.exit(1); }
  console.log("secret patterns ok " + SAMPLES.length + "/" + SAMPLES.length);
  process.exit(0);
}

const failures = selfTest();
if (failures.length) { console.error("secret self-test failed: " + failures.join(", ")); console.log("RESULT: FAIL"); process.exit(1); }

const hits = [];
try {
  for (const path of stagedPaths()) {
    if (/\.(glb|ktx2?|jpg|jpeg|png|bin|wasm|zip|gz)$/i.test(path)) continue;
    for (const line of addedLines(path)) for (const name of matchLine(line)) hits.push(path + ": " + name);
  }
} catch (error) {
  console.error("secret scan could not read the staged diff: " + error.message);
  console.log("RESULT: FAIL");
  process.exit(1);
}

if (hits.length) {
  for (const h of hits) console.error("secret-like text: " + h);
  console.log("RESULT: FAIL");
  process.exit(1);
}
console.log("RESULT: PASS");
