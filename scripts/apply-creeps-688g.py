# Applies scenarios/moba/creeps_688g.tsv to the neutral and Roshan unit lines of units.osd, and drops the
# placeholder heroes that have no 6.88g hero (DROP). Run: python3 -I scripts/apply-creeps-688g.py
import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
DROP = {"ironhide", "swiftbow", "stormcaller", "bulwark", "shadeblade", "emberlord", "gale_archer",
        "frostmage", "tidecaller", "thornwarden"}
rows = [r for r in csv.reader(
    (line for line in (ROOT / "creeps_688g.tsv").read_text().splitlines() if not line.startswith("#")), delimiter="\t")]
hdr, data = rows[0], rows[1:]
by_id = {r[0]: dict(zip(hdr, r)) for r in data}

def unit_line(line):
    m = re.match(r'unit\("(\w+)"', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    line = re.sub(r'^unit\("(\w+)", \d+, \d+, \d+',
                  lambda g: f'unit("{g.group(1)}", {v["hp"]}, {v["spd"]}, {v["radius"] or g.group(0).rsplit(", ", 1)[1]}', line)
    subs = {"weapon_damage": v["damage"], "weapon_range_milli": v["range_milli"], "weapon_period": v["period"],
            "sight_milli": v["sight_milli"], "health_regen": v["regen_milli"]}
    if v["armor"]:
        subs["armor"] = v["armor"]
    for key, val in subs.items():
        line = re.sub(rf'(\b{key}=)-?\d+', lambda g: f"{g.group(1)}{val}", line)
    return line

def keep(line):
    m = re.match(r'(unit|unit_extra|hero)\("(\w+)"', line) or re.match(r'hero (\w+) ', line)
    return not (m and m.group(m.lastindex) in DROP)

units = ROOT / "units.osd"
text = [unit_line(l) for l in units.read_text().split("\n") if keep(l)]
units.write_text("\n".join(text))
for name in ("units_extra.osd", "attr_rules.txt"):
    path = ROOT / name
    path.write_text("\n".join(l for l in path.read_text().split("\n") if keep(l)))
print(f"applied {len(by_id)} creep rows, dropped {len(DROP)} placeholders", file=sys.stderr)
