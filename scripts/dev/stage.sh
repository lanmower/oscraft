#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-$root/.dev-site}
rust="$root/rust"

cd "$rust"
if [ "${WASM:-0}" = 1 ] || [ ! -f crates/web/site/pkg/web_bg.wasm ]; then
  wasm-pack build crates/web --target web --out-dir site/pkg
fi
for p in moba duel-routes; do
  img=$(node -p "require('$root/scenarios/$p/manifest.json').image")
  cargo run -q -p mapexport -- "$root/scenarios/$p" "$root/scenarios/$p/$img"
done

bash "$root/scripts/layout-site.sh" "$out" local local
echo "serve: node scripts/dev/serve.mjs $out"
