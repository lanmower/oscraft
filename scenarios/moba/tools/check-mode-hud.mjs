import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const moba = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (name) => fs.readFileSync(path.join(moba, name), "utf8");
const external = fs.readdirSync(moba).filter((f) => f.endsWith(".lua") && f !== "modes.lua").map(read).join("\n");
const modesLua = read("modes.lua");
const draftLua = read("draft.lua");
const ui = JSON.parse(read("moba_ui.json")).text;
const supported = [...modesLua.match(/local SUPPORTED = \{([^}]*)\}/)[1].matchAll(/(\w+) = true/g)].map((m) => m[1]);
const lobbyButtons = modesLua.match(/modes\.BUTTONS = \{([^}]*)\}/)[1].match(/"(\w+)"/g).map((s) => s.slice(1, -1));
const chatInfo = new Set([...read("chat_text.lua").matchAll(/\{ "(\w+)", \{ "\|c/g)].map((m) => m[1]));

const HELPER_CALLS = { ar: "picks_free|assign", aa: "allows", ai: "allows", as: "allows", ro: "allows", mo: "allows", du: "duplicates", sh: "duplicates|assign", fr: "respawn" };
const DRAFT_KINDS = ["cm", "cd", "rd", "sd", "rv"];
const SETTINGS = {
  em: ["unit_rules.lua", /EASY_TOWER_TAKES_PCT = (\d+)/, "towers take %"],
  fr: ["modes.lua", /math\.floor\(secs \/ (\d+)\)/, "respawn divisor"],
  mi: ["mode_rules.lua", /MINI_SCALE_PERCENT = (\d+)/, "hero scale %"],
  id: ["mode_rules.lua", /ITEM_DROP_SLOTS = (\d+)/, "drop slots"],
  x: ["draft.lua", /CD_POOL_X = (\d+)/, "extended pool"],
};

function settingOf(code) {
  const spec = SETTINGS[code];
  if (!spec) return "-";
  const found = read(spec[0]).match(spec[1]);
  return found ? `${spec[2]} ${found[1]}` : "missing";
}

function gateOf(code) {
  if (code === "ap") return "default";
  if (DRAFT_KINDS.includes(code)) return draftLua.includes(`${code} = ${code}`) ? "draft" : "missing";
  if (HELPER_CALLS[code]) {
    const called = new RegExp(`modes\\.(${HELPER_CALLS[code]})\\(`).test(external);
    return modesLua.includes(`FLAGS.${code}`) && called ? "modes" : "missing";
  }
  return external.includes(`modes.has("${code}")`) ? "has" : "missing";
}

function modeLine(codes) {
  const shown = codes.map((c) => ui.lobby.modes[c] ?? c);
  return shown.length === 0 ? `${ui.hud.mode} ${ui.hud.defaultMode}` : `${ui.hud.mode} ${shown.join(", ")}`;
}

const failures = [];
const table = supported.map((code) => {
  const name = ui.lobby.modes[code];
  const rule = ui.modeRules[code];
  const gate = gateOf(code);
  if (!name) failures.push(`${code}: no lobby label`);
  if (!rule) failures.push(`${code}: no HUD rule`);
  if (!chatInfo.has(code)) failures.push(`${code}: no chat gameInfo row`);
  if (gate === "missing") failures.push(`${code}: gameplay setting not applied`);
  return { code, name: name ?? "MISSING", lobby: lobbyButtons.includes(code) ? "yes" : "no", hudName: name && rule ? "yes" : "no", chatRule: chatInfo.has(code) ? "yes" : "no", applied: gate, setting: settingOf(code) };
});
console.table(table);

for (const codes of [[], ["ar", "fr"], ["em", "mi", "id"], ["cd", "x"], ["aa", "du", "sc"]]) {
  console.log(`${modeLine(codes).padEnd(48)} | ${codes.map((c) => `${c}: ${settingOf(c)}`).join("; ") || "-"}`);
}

if (failures.length > 0) {
  for (const f of failures) console.error(`FAIL ${f}`);
  process.exitCode = 1;
} else {
  console.log(`ok ${supported.length} modes`);
}
