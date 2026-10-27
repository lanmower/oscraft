#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:?usage: layout-site.sh <out-dir> <ref> <run>}
ref=${2:-local}
run=${3:-local}
sha=${GITHUB_SHA:-$(git -C "$root" rev-parse HEAD 2>/dev/null || echo local)}
cd "$root"

rm -rf "$out" && mkdir -p "$out/js"
cp -r js/client "$out/js/client"
rm -rf "$out/js/client/pkg" && cp -r rust/crates/web/site/pkg "$out/js/client/pkg"
rm -f "$out/js/client/pkg/.gitignore"
vendor="$out/js/client/vendor"
rm -rf "$vendor/wireweave/node_modules" "$vendor/wireweave/site" "$vendor/wireweave"/.[a-z]* "$vendor/wireweave"/scratch-verify-*.mjs
rm -f "$vendor/wireweave"/*.md "$vendor/wireweave"/package*.json "$vendor/wireweave/flatspace.config.mjs"
rm -f "$vendor"/three/three.*.js
cp -r scenarios "$out/scenarios"
rm -rf "$out"/scenarios/*/tools "$out"/scenarios/*/provenance
mkdir -p "$out/scenarios/moba/assets/models"
cp rust/crates/web/site/assets/units/*.glb "$out/scenarios/moba/assets/models/"
node scripts/assets/build-packs.mjs scenarios/moba/assets "$out/scenarios/moba/assets/packs"
cp -r mods "$out/mods"
printf '{"sha":"%s","ref":"%s","run":"%s","built":"%s"}\n' "$sha" "$ref" "$run" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$out/version.json"
cp "$out/version.json" "$out/js/client/version.json"
printf '<meta http-equiv="refresh" content="0; url=js/client/play.html">\n' > "$out/index.html"
for f in version.json js/client/editor.html js/client/play.html js/client/play.js js/client/pack_load.js scenarios/moba/manifest.json scenarios/moba/moba.osdt js/client/pkg/web.js js/client/pkg/web_bg.wasm js/client/vendor/wireweave/src/index.js js/client/vendor/three/libs/draco/draco_wasm_wrapper.js; do
  test -f "$out/$f" || { echo "layout-site: missing $f" >&2; exit 1; }
done

if [ "${SKIP_PACKS:-0}" != 1 ]; then
  node scripts/pack-package.mjs scenarios/moba "$out/packs"
  node scripts/pack-latest.mjs "$out/packs"
  test -f "$out/packs/latest.json" || { echo "layout-site: missing packs/latest.json" >&2; exit 1; }
fi
echo "laid out $out sha=$sha ref=$ref size=$(du -sk "$out" | cut -f1)KiB"
