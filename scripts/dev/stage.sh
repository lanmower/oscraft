#!/usr/bin/env bash
# Stages the GitHub Pages site locally (same layout as .github/workflows/pages.yml) into .dev-site.
# Usage: scripts/dev/stage.sh [site-dir]   Set WASM=1 to force a wasm-pack rebuild.
# Needs: cargo, wasm-pack and the wasm32-unknown-unknown target (only when the pkg is missing or WASM=1).
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$root/.dev-site}
rust="$root/rust"
sha=$(git -C "$root" rev-parse HEAD 2>/dev/null || echo local)

cd "$rust"
if [ "${WASM:-0}" = 1 ] || [ ! -f crates/web/site/pkg/web_bg.wasm ]; then
  wasm-pack build crates/web --target web --out-dir site/pkg
fi
# Each pack builds its map image into its own directory (manifest.image); the site serves the packs from scenarios/.
for p in moba duel-routes; do
  img=$(node -p "require('$root/scenarios/$p/manifest.json').image")
  cargo run -q -p mapexport -- "$root/scenarios/$p" "$root/scenarios/$p/$img"
done

cd "$root"
rm -rf "$out" && mkdir -p "$out/js"
cp -r js/client "$out/js/client"
rm -rf "$out/js/client/pkg" && cp -r rust/crates/web/site/pkg "$out/js/client/pkg"
rm -f "$out/js/client/pkg/.gitignore"
cp -r scenarios "$out/scenarios"
mkdir -p "$out/scenarios/moba/assets/models"
cp rust/crates/web/site/assets/units/*.glb "$out/scenarios/moba/assets/models/"
cp -r mods "$out/mods"
printf '{"sha":"%s","ref":"local","run":"local","built":"%s"}\n' "$sha" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$out/version.json"
cp "$out/version.json" "$out/js/client/version.json"
printf '<meta http-equiv="refresh" content="0; url=js/client/play.html">\n' > "$out/index.html"
for f in js/client/editor.html js/client/play.html js/client/play.js js/client/pack_load.js scenarios/moba/manifest.json scenarios/moba/moba.osdt js/client/pkg/web.js js/client/pkg/web_bg.wasm; do
  test -f "$out/$f" || { echo "stage: missing $f" >&2; exit 1; }
done
echo "staged $out sha=$sha"
echo "serve: node scripts/dev/serve.mjs $out"
