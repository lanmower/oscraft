#!/usr/bin/env python3
import base64
import io
import json
import math
import os
import sys

from PIL import Image


def resized_tile(path, cell):
    with Image.open(path) as source:
        return source.convert("RGBA").resize((cell, cell), Image.LANCZOS)


def pack_category_atlas(spec):
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
        atlas.paste(resized_tile(icon["path"], cell), (x, y))
        cells[icon["name"]] = [x, y, cell]
    out_dir = spec["out_dir"]
    os.makedirs(out_dir, exist_ok=True)
    name = spec["name"]
    atlas.save(os.path.join(out_dir, name + ".png"), "PNG")
    atlas.save(os.path.join(out_dir, name + ".webp"), "WEBP", quality=quality, method=6)
    return {"width": cols * cell, "height": rows * cell, "cols": cols, "rows": rows, "cell": cell, "icons": cells}


def encode_cell_webps(spec):
    cell = int(spec["cell"])
    quality = int(spec["quality"])
    encoded = {}
    for icon in sorted(spec["icons"], key=lambda icon: icon["name"]):
        buffer = io.BytesIO()
        resized_tile(icon["path"], cell).save(buffer, "WEBP", quality=quality, method=6)
        encoded[icon["name"]] = base64.b64encode(buffer.getvalue()).decode("ascii")
    return {"cells": encoded}


if __name__ == "__main__":
    spec = json.load(sys.stdin)
    result = encode_cell_webps(spec) if spec.get("mode") == "cells" else pack_category_atlas(spec)
    json.dump(result, sys.stdout)
