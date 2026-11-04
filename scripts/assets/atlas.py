#!/usr/bin/env python3
import json
import math
import os
import sys

from PIL import Image


def pack_category_atlas():
    spec = json.load(sys.stdin)
    cell = int(spec["cell"])
    quality = int(spec["quality"])
    icons = sorted(spec["icons"], key=lambda icon: icon["name"])
    cols = max(1, math.ceil(math.sqrt(len(icons))))
    rows = math.ceil(len(icons) / cols)
    atlas = Image.new("RGBA", (cols * cell, rows * cell), (0, 0, 0, 0))
    cells = {}
    for index, icon in enumerate(icons):
        x = (index % cols) * cell
        y = (index // cols) * cell
        with Image.open(icon["path"]) as source:
            tile = source.convert("RGBA").resize((cell, cell), Image.LANCZOS)
        atlas.paste(tile, (x, y))
        cells[icon["name"]] = [x, y, cell]
    out_dir = spec["out_dir"]
    os.makedirs(out_dir, exist_ok=True)
    name = spec["name"]
    atlas.save(os.path.join(out_dir, name + ".png"), "PNG")
    atlas.save(os.path.join(out_dir, name + ".webp"), "WEBP", quality=quality, method=6)
    json.dump({"width": cols * cell, "height": rows * cell, "cols": cols, "rows": rows, "cell": cell, "icons": cells}, sys.stdout)


if __name__ == "__main__":
    pack_category_atlas()
