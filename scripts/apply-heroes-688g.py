import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
MILLI, TPS = 15.625, 16
rows = [r for r in csv.reader(
    (ROOT / "heroes_688g.tsv").read_text().splitlines(), delimiter="\t")]
hdr, data = rows[0], rows[1:]
by_id = {r[0]: dict(zip(hdr, r)) for r in data}
FLAG = "UNVERIFIED "
PROVENANCE = ROOT / "provenance" / "units_heroes.osd.tsv"

def kept(v):
    flag = v["unverified"].strip()
    return None if flag == "all" else set(filter(None, flag.split(",")))

def unit_line(line):
    m = re.match(r'unit\("(\w+)"', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    keep = kept(v)
    if keep is None:
        return line
    unit_fields = {"hp": lambda: v["hp"], "spd": lambda: round(float(v["spd"]) * MILLI / TPS),
                   "collision": lambda: round(float(v["collision"]) * MILLI)}
    cur = re.match(r'unit\("(\w+)", (-?\d+), (-?\d+), (-?\d+)', line)
    head = [cur.group(2), cur.group(3), cur.group(4)]
    for i, col in enumerate(("hp", "spd", "collision")):
        if col not in keep:
            head[i] = str(unit_fields[col]())
    line = re.sub(r'^unit\("(\w+)", -?\d+, -?\d+, -?\d+', lambda g: f'unit("{g.group(1)}", {", ".join(head)}', line)
    subs = {
        "weapon_damage": ("damage", lambda: round(int(v["damage"]) / 100)),
        "weapon_range_milli": ("range", lambda: round(float(v["range"]) * MILLI)),
        "weapon_period": ("bat", lambda: max(1, round(int(v["bat"]) * TPS / 100))),
        "sight_milli": ("sight", lambda: round(float(v["sight"]) * MILLI)),
        "defense": ("armor", lambda: round(int(v["armor"]) / 100)),
    }
    for key, (col, val) in subs.items():
        if col not in keep:
            line = re.sub(rf'(\b{key}=)-?\d+', lambda g: f"{g.group(1)}{val()}", line)
    return line

def hero_line(line):
    m = re.match(r'attr (\w+) ', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    keep = kept(v)
    if keep is None:
        return line
    old = dict(re.findall(r'(\w+)=(-?\d+)', line))
    cols = ("primary", "str", "str_gain", "agi", "agi_gain", "int", "int_gain", "mana", "mana_regen", "hp_regen", "armor")
    fields = [(c, old[c] if c in keep else v[c]) for c in cols]
    fields += [("mr", old.get("mr", "25")), ("bat", old["bat"] if "bat" in keep else v["bat"]),
               ("damage", old["damage"] if "damage" in keep else v["damage"])]
    return f"attr {m.group(1)} " + " ".join(f"{k}={x}" for k, x in fields)

def flag_comment(hero):
    v = by_id[hero]
    if kept(v) is None:
        return f"{FLAG}{hero}: not in 6.88g (no hero unit, ability set or pick-table entry); every stat keeps its reference ruleset value"
    if not kept(v):
        return None
    return f"{FLAG}{hero}: {v['note']}"

units = ROOT / "units_heroes.osd"
out = []
flags = {}
for line in units.read_text().split("\n"):
    new = unit_line(line)
    m = re.match(r'unit\("(\w+)"', line)
    if m and m.group(1) in by_id:
        comment = flag_comment(m.group(1))
        if comment:
            flags[m.group(1)] = comment
    out.append(new)
units.write_text("\n".join(out))

def sidecar_rows():
    lines = PROVENANCE.read_text().split("\n")
    return lines[0], [l for l in lines[1:] if l]

def is_flag_row(row):
    anchor, kind, note = row.split("\t", 2)
    return kind == "unverified" and note.startswith(f"{FLAG}{anchor}: ") and anchor in by_id

header, existing = sidecar_rows()
carried = {r.split("\t", 1)[0]: r for r in existing if is_flag_row(r)}
retained = [r for r in existing if not is_flag_row(r)]
order = {}
for line in out:
    m = re.match(r'unit\("(\w+)"', line)
    if m and m.group(1) not in order:
        order[m.group(1)] = len(order)
merged = [(order.get(r.split("\t", 1)[0], -1), 0, r) for r in retained]
merged += [(order[h], 1, carried.get(h, f"{h}\tunverified\t{c}")) for h, c in flags.items()]
merged.sort(key=lambda t: (t[0], t[1]))
PROVENANCE.write_text(header + "\n" + "\n".join(t[2] for t in merged) + "\n")
rules = ROOT / "attr_rules.txt"
rules.write_text("\n".join(hero_line(l) for l in rules.read_text().split("\n")))
print(f"applied {len(by_id)} heroes", file=sys.stderr)
