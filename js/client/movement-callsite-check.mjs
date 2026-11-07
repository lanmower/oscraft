import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const read = (name) => readFileSync(resolve(here, name), "utf8");
const fail = (what) => { throw new Error(what); };
const productionFiles = readdirSync(here).filter((name) => name.endsWith(".js"));
const sources = Object.fromEntries(productionFiles.map((name) => [name, read(name)]));
const play = sources["play.js"];
const focus = sources["game_focus.js"];

const callLines = (pattern) => {
  const hits = [];
  for (const [name, text] of Object.entries(sources)) {
    text.split("\n").forEach((line, i) => {
      if (pattern.test(line) && !/^\s*(export\s+)?function\s/.test(line) && !/^\s*import\s/.test(line)) hits.push(`${name}:${i + 1}`);
    });
  }
  return hits;
};

const functionBody = (text, header) => {
  const start = text.indexOf(header);
  if (start < 0) fail(`missing function header: ${header}`);
  const end = text.indexOf("\n}\n", start);
  if (end < 0) fail(`unterminated function: ${header}`);
  return text.slice(start, end + 3);
};

const requireIn = (text, needle, what) => { if (!text.includes(needle)) fail(`${what}: missing ${needle}`); };

const stepHits = callLines(/stepHeroRun\(/);
if (stepHits.length !== 1 || !stepHits[0].startsWith("play.js:")) fail(`stepHeroRun has ${stepHits.length} production call sites, want 1 in play.js: ${stepHits.join(", ")}`);
const driveBody = functionBody(play, "function driveLockedHero(dtS) {");
requireIn(driveBody, "const shown = drawnPosition(hero);", "driveLockedHero");
requireIn(driveBody, "lockedFocus = approachTarget(lockedFocus, followTarget(shown, groundAt), 1 - Math.exp(-dtS * LOCK_FOLLOW_RATE));", "driveLockedHero");
requireIn(driveBody, "renderer3d.setCamera({ target: lockedFocus });", "driveLockedHero");
requireIn(driveBody, "stepHeroRun(heroRun, {", "driveLockedHero");

const panBody = functionBody(play, "function panByKeys(dtS) {");
requireIn(panBody, "if (heroViewLock.isLocked()) {", "panByKeys");
requireIn(panBody, "driveLockedHero(dtS);", "panByKeys");
requireIn(panBody, "heroRun = createHeroRun();", "panByKeys");
requireIn(panBody, "lockedFocus = null;", "panByKeys");

const approachHits = callLines(/approachTarget\(/);
if (approachHits.length !== 1 || !approachHits[0].startsWith("play.js:")) fail(`approachTarget has ${approachHits.length} production call sites, want 1 in play.js: ${approachHits.join(", ")}`);

const stepCalls = callLines(/modUi\.step\(/);
if (stepCalls.length !== 1) fail(`modUi.step has ${stepCalls.length} call sites, want 1: ${stepCalls.join(", ")}`);
requireIn(play, "modUi.step(dtS, heroViewLock.isLocked());", "frameStep");
requireIn(sources["modui.js"], "function step(dtS, locked = false) {", "modui.js step signature");
requireIn(sources["modui.js"], "if (!r || locked) { move = null; return; }", "modui.js step lock gate");

requireIn(play, "installGameFocusGuard({ onArrow: (key) => keysDown.add(key), onArrowUp: (key) => keysDown.delete(key) });", "play.js focus guard install");
requireIn(focus, "export function installGameFocusGuard({ onArrow, onArrowUp }) {", "game_focus.js signature");
requireIn(focus, "capturedArrows.add(e.key);", "keydown capture");
requireIn(focus, "onArrow(e.key);", "keydown capture");
requireIn(focus, "if (!capturedArrows.delete(e.key)) return;", "keyup release");
requireIn(focus, "onArrowUp(e.key);", "keyup release");
requireIn(focus, "for (const key of capturedArrows) onArrowUp(key);", "blur release");
requireIn(focus, "if (!isLive() || !e.key.startsWith(\"Arrow\") || isTextEntry(e.target)) return;", "keydown capture predicate");
requireIn(play, "keysDown.delete(e.key);", "window keyup fallback");

console.log(`movement call-site ok: stepHeroRun 1 site (${stepHits[0]}), approachTarget 1 site (${approachHits[0]}), modUi.step 1 site (${stepCalls[0]}), keyup and blur release paired with keydown capture`);
