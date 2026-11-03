import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
MILLI, TPS = 15.625, 16
rows = [r for r in csv.reader(
    (ROOT / "heroes_688g.tsv").read_text().splitlines(), delimiter="\t")]
hdr, data = rows[0], rows[1:]
by_id = {r[0]: dict(zip(hdr, r)) for r in data}

def kept(v):
    flag = v["keep_ref"].strip()
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

units = ROOT / "units_heroes.osd"
out = [unit_line(line) for line in units.read_text().split("\n")]
units.write_text("\n".join(out))
rules = ROOT / "attr_rules.txt"
rules.write_text("\n".join(hero_line(l) for l in rules.read_text().split("\n")))
print(f"applied {len(by_id)} heroes", file=sys.stderr)
