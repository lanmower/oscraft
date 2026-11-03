import csv, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "scenarios" / "moba"
rows = [r for r in csv.reader(
    (line for line in (ROOT / "abilities_688g.tsv").read_text().splitlines() if not line.startswith("#")), delimiter="\t")]
hdr, data = rows[0], rows[1:]
applied = {r[0]: dict(zip(hdr, r)) for r in data if dict(zip(hdr, r))["status"] == "applied"}

def ability_line(line):
    m = re.match(r'ability\("(\w+)", (\d+)', line)
    if not m or m.group(1) not in applied:
        return line
    return re.sub(r'^ability\("(\w+)", \d+', lambda g: f'ability("{g.group(1)}", {applied[g.group(1)]["cool_ticks"]}', line, count=1)

path = ROOT / "abilities.osd"
path.write_text("\n".join(ability_line(l) for l in path.read_text().split("\n")))
print(f"applied {len(applied)} abilities", file=sys.stderr)
