#!/usr/bin/env python3
# Packs one category of square icons into an atlas of fixed cells, for scripts/assets/build-packs.mjs. Reads one JSON object
# on stdin: {"name", "cell", "quality", "out_dir", "icons": [{"name", "path"}]}. Icons are sorted by name and laid out
# row by row in ceil(sqrt(n)) columns, each downscaled to cell x cell, so the same inputs always give the same bytes.
# Writes <out_dir>/<name>.png (lossless, the source for the KTX2 encode) and <out_dir>/<name>.webp, and prints one JSON
# object: width, height, cols, rows, cell and icons as {name: [x, y, cell]}.
import json
import math
import os
import sys

from PIL import Image


def main():
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
    main()
