#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const mobaDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SKILL_RANKS = 4;
const TICKS = 16;
const MILLI = 15.625;
const DURATION_FLOOR_S = 0.1;

const lines = (file) => fs.readFileSync(file, 'utf8').split(/\r?\n/);
const intList = (s) => s.split(',').map((v) => Number(v.trim())).filter((v) => !Number.isNaN(v));
const pipeList = (s) => (s === '' ? null : s.split('|').map(Number));

function parseOsd(files) {
  const abilities = new Map();
  const get = (n) => {
    if (!abilities.has(n)) abilities.set(n, { name: n, scaled: new Map(), effects: [] });
    return abilities.get(n);
  };
  for (const file of files) {
    for (const line of lines(file)) {
      let m = /^ability\("([^"]+)", (-?\d+), (-?\d+), "([^"]*)", \[(.*)\]\)\s*$/.exec(line);
      if (m) {
        const a = get(m[1]);
        a.file = path.basename(file);
        a.cooldown = Number(m[2]);
        a.mana = Number(m[3]);
        a.target = m[4];
        a.effects = [...m[5].matchAll(/effect\("([^"]+)"((?:, p\d=-?\d+)*)\)/g)].map((e) => ({
          kind: e[1],
          p: [0, 1, 2, 3].map((i) => {
            const v = new RegExp(`p${i}=(-?\\d+)`).exec(e[2]);
            return v ? Number(v[1]) : 0;
          }),
        }));
        continue;
      }
      m = /^skill\("([^"]+)", ultimate=(True|False), gates=\[([^\]]*)\], damage=\[([^\]]*)\], cooldown=\[([^\]]*)\], energy=\[([^\]]*)\]\)/.exec(line);
      if (m) {
        Object.assign(get(m[1]), { ultimate: m[2] === 'True', gates: intList(m[3]), skillDamage: intList(m[4]), skillCooldown: intList(m[5]), skillEnergy: intList(m[6]) });
        continue;
      }
      m = /^ability_ext\("([^"]+)", (.*)\)\s*$/.exec(line);
      if (m) {
        const r = /cast_range=(\d+)/.exec(m[2]);
        if (r) get(m[1]).castRange = Number(r[1]);
        continue;
      }
      m = /^ability_scaled\("([^"]+)", (\d+), (\d+), \[([^\]]*)\]\)/.exec(line);
      if (m) get(m[1]).scaled.set(`${m[2]},${m[3]}`, intList(m[4]));
    }
  }
  return abilities;
}

function readTsv(file) {
  if (!fs.existsSync(file)) return [];
  const rows = lines(file).filter((l) => l !== '' && !l.startsWith('#'));
  const head = rows[0].split('\t');
  return rows.slice(1).map((l) => {
    const c = l.split('\t');
    return Object.fromEntries(head.map((h, i) => [h, c[i] ?? '']));
  });
}

const durationTicks = (a, level) => {
  const out = [];
  const pick = (i, p, base) => {
    const s = a.scaled.get(`${i},${p}`);
    out.push(s ? s[Math.min(level, s.length - 1)] : base);
  };
  a.effects.forEach((e, i) => {
    if (e.kind === 'timed_stat' || e.kind === 'dot') pick(i, 2, e.p[2]);
    else if (e.kind === 'status') pick(i, 1, e.p[1]);
    else if (['stun', 'silence', 'root', 'invisibility', 'slow'].includes(e.kind)) pick(i, 0, e.p[0]);
    else if (e.kind === 'zone') pick(i, 2, e.p[2]);
  });
  return out;
};

const damageValues = (a, level) => {
  const out = [];
  a.effects.forEach((e, i) => {
    if (e.kind !== 'damage' && e.kind !== 'dot' && e.kind !== 'chain') return;
    const s = a.scaled.get(`${i},0`);
    out.push(s ? s[Math.min(level, s.length - 1)] : e.p[0]);
  });
  return out;
};

const osdFiles = fs.readdirSync(mobaDir).filter((n) => /^abilities.*\.osd$/.test(n)).sort().map((n) => path.join(mobaDir, n));
const abilities = parseOsd(osdFiles);
const reference = readTsv(path.join(mobaDir, 'abilities_688g_reference.tsv'));
const allowRows = readTsv(path.join(mobaDir, 'abilities_688g_reference.allow.tsv'));
const allow = new Map(allowRows.map((r) => [`${r.ability}\t${r.field}\t${r.level}`, { ...r, used: false }]));

const mismatches = [];
const report = (ref, field, level, got, want) => {
  const keys = [`${ref.oscraft_ability}\t${field}\t${level}`, `${ref.oscraft_ability}\t${field}\t*`];
  const hit = keys.map((k) => allow.get(k)).find(Boolean);
  if (hit) {
    hit.used = true;
    return;
  }
  const src = { cooldown: ref.cooldown_line, mana: ref.mana_line, range: ref.range_line, duration: ref.duration_line || ref.hero_duration_line, damage: ref.damage_line, gates: ref.req_line || ref.ini_line, levels: ref.ini_line }[field] || ref.ini_line;
  mismatches.push(`${ref.oscraft_ability}\t${field}\tL${level}\toscraft=${got}\t6.88g=${want}\tability.ini:${src} (${ref.code})`);
};

let checked = 0;
for (const ref of reference) {
  const a = abilities.get(ref.oscraft_ability);
  if (!a) {
    report(ref, 'ability', '*', 'absent', ref.code);
    continue;
  }
  const levels = Math.min(Number(ref.levels) || 0, SKILL_RANKS);
  const req = ref.req_level === '' ? 1 : Number(ref.req_level);
  const skip = ref.level_skip === '' ? 2 : Number(ref.level_skip);

  if (a.gates) {
    checked++;
    if (a.skillCooldown.length !== levels && levels > 0) report(ref, 'levels', '*', a.skillCooldown.length, levels);
    const want = Array.from({ length: levels }, (_, i) => req + skip * i);
    if (want.join() !== a.gates.slice(0, levels).join()) report(ref, 'gates', '*', `[${a.gates}]`, `[${want}] (reqLevel ${req} levelSkip ${skip})`);
  }

  const cool = pipeList(ref.cooldown_s);
  if (cool) {
    const have = a.skillCooldown ?? [a.cooldown];
    cool.forEach((s, i) => {
      checked++;
      const want = Math.round(s * TICKS);
      const got = have[Math.min(i, have.length - 1)];
      if (got !== want) report(ref, 'cooldown', i + 1, got, `${want} ticks (${s} s)`);
    });
  }

  const mana = pipeList(ref.mana);
  if (mana) {
    const have = a.skillEnergy ?? [a.mana];
    mana.forEach((v, i) => {
      checked++;
      const got = have[Math.min(i, have.length - 1)];
      if (got !== v) report(ref, 'mana', i + 1, got, v);
    });
  }

  const range = pipeList(ref.range);
  if (range && a.castRange !== undefined) {
    const have = a.scaled.get('252,0') ?? [a.castRange];
    range.forEach((v, i) => {
      checked++;
      const want = Math.round(v * MILLI);
      const got = have[Math.min(i, have.length - 1)];
      if (Math.abs(got - want) > 1) report(ref, 'range', i + 1, got, `${want} milli (${v} units)`);
    });
  }

  const dur = pipeList(ref.duration_s);
  const heroDur = pipeList(ref.hero_duration_s);
  if ((dur || heroDur) && a.effects.length) {
    const n = Math.max(dur?.length ?? 0, heroDur?.length ?? 0);
    for (let i = 0; i < n; i++) {
      const wants = [dur?.[i], heroDur?.[i]].filter((v) => v !== undefined && v >= DURATION_FLOOR_S).map((v) => Math.round(v * TICKS));
      const have = durationTicks(a, i);
      if (!wants.length || !have.length) continue;
      checked++;
      if (!have.some((t) => wants.includes(t))) report(ref, 'duration', i + 1, `[${[...new Set(have)]}] ticks`, `${[...new Set(wants)]} ticks (Dur ${dur?.[i] ?? '-'} s, HeroDur ${heroDur?.[i] ?? '-'} s)`);
    }
  }

  const damage = ref.damage === '' ? [] : ref.damage.split(';').map((f) => pipeList(f.slice(f.indexOf('=') + 1)));
  if (damage.length && damageValues(a, 0).length) {
    const n = Math.max(...damage.map((d) => d.length));
    for (let i = 0; i < n; i++) {
      const have = damageValues(a, i);
      const wants = damage.map((d) => d[Math.min(i, d.length - 1)]);
      checked++;
      if (!have.some((v) => wants.includes(v))) report(ref, 'damage', i + 1, `[${have}]`, `${[...new Set(wants)]} (Data ${ref.damage.split(';').map((f) => f[0])})`);
    }
  }
}

const stale = [...allow.values()].filter((r) => !r.used);
for (const r of allowRows) if (!r.reason || r.reason.trim() === '') mismatches.push(`allow list entry without reason: ${r.ability}\t${r.field}\t${r.level}`);
for (const r of stale) mismatches.push(`unused allow list entry: ${r.ability}\t${r.field}\t${r.level}`);

if (mismatches.length) {
  console.error(`check-ability-balance: ${mismatches.length} problems (ability, field, level, oscraft value, 6.88g value, source)`);
  for (const m of mismatches) console.error(m);
  process.exit(1);
}
console.log(`check-ability-balance: ok, ${reference.length} abilities, ${checked} values checked, ${allowRows.length} allow-listed`);
