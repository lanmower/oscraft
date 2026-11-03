import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
DRY_RUN = "--dry-run" in sys.argv
rows = [r for r in csv.reader(
    (line for line in (ROOT / "creeps_688g.tsv").read_text().splitlines() if not line.startswith("#")), delimiter="\t")]
hdr, data = rows[0], rows[1:]
by_id = {r[0]: dict(zip(hdr, r)) for r in data}
defense_set = 0

def unit_line(line):
    global defense_set
    m = re.match(r'unit\(\"(\w+)\"', line)
    if not m or m.group(1) not in by_id:
        return line
    v = by_id[m.group(1)]
    line = re.sub(r'^unit\(\"(\w+)\", \d+, \d+, \d+',
                  lambda g: f'unit("{g.group(1)}", {v["hp"]}, {v["spd"]}, {v["radius"] or g.group(0).rsplit(", ", 1)[1]}', line)
    subs = {"weapon_damage": v["damage"], "weapon_range_milli": v["range_milli"], "weapon_period": v["period"],
            "sight_milli": v["sight_milli"], "health_regen": v["regen_milli"]}
    if v.get("abilities"):
        names = ", ".join(f'"{name}"' for name in v["abilities"].split(","))
        if "abilities=[" in line:
            line = re.sub(r'abilities=\[[^\]]*\]', f"abilities=[{names}]", line)
        else:
            line = line.replace(", filters=", f", abilities=[{names}], filters=", 1)
    if v["armor"]:
        subs["defense"] = v["armor"]
    for key, val in subs.items():
        line, hits = re.subn(rf'(\b{key}=)-?\d+', lambda g: f"{g.group(1)}{val}", line)
        if key == "defense":
            defense_set += hits
    return line

units = ROOT / "units.osd"
old_lines = units.read_text().split("\n")
new_lines = [unit_line(l) for l in old_lines]
changed = sum(a != b for a, b in zip(old_lines, new_lines))
if DRY_RUN:
    print(f"dry run: units.osd {len(old_lines)} lines, {len(new_lines)} kept, {defense_set} defense= set, {changed} changed; nothing written", file=sys.stderr)
else:
    units.write_text("\n".join(new_lines))
    print(f"applied {len(by_id)} creep rows: {defense_set} defense= set, {changed} units.osd lines changed", file=sys.stderr)
