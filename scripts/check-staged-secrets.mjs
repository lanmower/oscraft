import { execFileSync } from "node:child_process";

const PATTERNS = [
  ["github token", /\b(gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})\b/],
  ["aws access key", /\bAKIA[0-9A-Z]{16}\b/],
  ["private key block", /-----BEGIN (RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ["bearer token", /\bBearer\s+[A-Za-z0-9._~+/-]{20,}=*/],
  ["generic secret", /\b(api[_-]?key|secret|token|password)\s*[:=]\s*["'][^"'\s]{16,}["']/i],
];

function stagedPaths() {
  const out = execFileSync("git", ["diff", "--cached", "--name-only", "--diff-filter=ACM"], { encoding: "utf8" });
  return out.split("\n").filter(Boolean);
}

function addedLines(path) {
  let diff;
  try {
    diff = execFileSync("git", ["diff", "--cached", "--unified=0", "--", path], { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 });
  } catch {
    return [];
  }
  return diff.split("\n").filter((l) => l.startsWith("+") && !l.startsWith("+++")).map((l) => l.slice(1));
}

const hits = [];
for (const path of stagedPaths()) {
  if (/\.(glb|ktx2?|jpg|jpeg|png|bin|wasm|zip|gz)$/i.test(path)) continue;
  for (const line of addedLines(path)) {
    for (const [name, re] of PATTERNS) {
      if (re.test(line)) hits.push(`${path}: ${name}`);
    }
  }
}

if (hits.length) {
  for (const h of hits) console.error(`secret-like text: ${h}`);
  console.log("RESULT: FAIL");
  process.exit(1);
}
console.log("RESULT: PASS");
