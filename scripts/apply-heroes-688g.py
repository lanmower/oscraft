# Applies scenarios/moba/heroes_688g.tsv to units_heroes.osd and the hero lines of hero_rules.txt.
# Heroes absent from the TSV keep their current lines. Scale: WC3 units to milli is 15.625 (sight, range, radius),
# speed is milli per tick at 16 ticks per second, weapon period is ticks. Run: python3 -I scripts/apply-heroes-688g.py
import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
MILLI, TPS = 15.625, 16
rows = [r for r in csv.reader(
    (line for line in (ROOT / "heroes_688g.tsv").read_text().splitlines() if not line.startswith("#")), delimiter="\t")]
hdr, data = rows[0], rows[1:]
by_id = {r[0]: dict(zip(hdr, r)) for r in data}

def unit_line(line, h):
    m = re.match(r'unit\("(\w+)"', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    speed = round(float(v["spd"]) * MILLI / TPS)
    line = re.sub(r'^unit\("(\w+)", \d+, \d+, \d+', lambda g: f'unit("{g.group(1)}", {v["hp"]}, {speed}, {round(float(v["collision"]) * MILLI)}', line)
    subs = {
        "weapon_damage": round(int(v["damage"]) / 100),
        "weapon_range_milli": round(float(v["range"]) * MILLI),
        "weapon_period": max(1, round(int(v["bat"]) * TPS / 100)),
        "sight_milli": round(float(v["sight"]) * MILLI),
        "armor": round(int(v["armor"]) / 100),
    }
    for key, val in subs.items():
        line = re.sub(rf'(\b{key}=)-?\d+', lambda g: f"{g.group(1)}{val}", line)
    return line

def hero_line(line):
    m = re.match(r'hero (\w+) ', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    old = dict(re.findall(r'(\w+)=(-?\d+)', line))
    fields = [("primary", v["primary"]), ("str", v["str"]), ("str_gain", v["str_gain"]), ("agi", v["agi"]),
              ("agi_gain", v["agi_gain"]), ("int", v["int"]), ("int_gain", v["int_gain"]), ("mana", v["mana"]),
              ("mana_regen", v["mana_regen"]), ("hp_regen", v["hp_regen"]), ("armor", v["armor"]),
              ("mr", old.get("mr", "25")), ("bat", v["bat"]), ("damage", v["damage"])]
    return f"hero {m.group(1)} " + " ".join(f"{k}={x}" for k, x in fields)

units = ROOT / "units_heroes.osd"
units.write_text("\n".join(unit_line(l, None) for l in units.read_text().split("\n")))
rules = ROOT / "hero_rules.txt"
rules.write_text("\n".join(hero_line(l) for l in rules.read_text().split("\n")))
print(f"applied {len(by_id)} heroes", file=sys.stderr)
