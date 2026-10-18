#!/usr/bin/env bash
# Builds one installable archive: oscraftd, oscraft-lsp, the data directory, the browser site and the browser client.
# Usage: scripts/package/build.sh <out-dir>
# Output: <out-dir>/oscraft-<version>.tar.gz, members at its root (bin/ data/ mods/ site/ js/client/ VERSION MANIFEST.sha256).
set -euo pipefail
out=${1:?usage: build.sh <out-dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
rust="$root/rust"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust/crates/server/Cargo.toml" | head -n 1)
stage="$out/stage"
rm -rf "$out"
mkdir -p "$stage/bin" "$stage/data" "$stage/site" "$stage/js/client/wire"

cd "$rust"
cargo build --release -q -p server --bin oscraftd
cargo build --release -q -p oscraft-lsp
cp target/release/oscraftd target/release/oscraft-lsp "$stage/bin/"
cp data/*.osd "$stage/data/"
cp -R crates/web/site/. "$stage/site/"
cargo run -q -p mapexport -- "$stage/site/map.bin" ../scenarios/moba
wasm-pack build crates/web --target web --out-dir "$stage/site/pkg" > /dev/null
cd "$root"
cp -R js/client/. "$stage/js/client/"
rm -rf "$stage/js/client/pkg" "$stage/js/client/editor-check.mjs"
cp -R mods "$stage/mods"
for page in play editor replay; do
  [ -f "js/client/$page.html" ] || [ "$page" = replay ] || { echo "missing js/client/$page.html" >&2; exit 1; }
done
wasm-pack build rust/crates/web --target web --out-dir "$stage/js/client/pkg" > /dev/null
cp "$stage/site/map.bin" "$stage/js/client/map.bin"

printf '%s\n' "$version" > "$stage/VERSION"
(cd "$stage" && find . -type f ! -name MANIFEST.sha256 | sed 's|^\./||' | LC_ALL=C sort | while read -r f; do
  if command -v sha256sum > /dev/null; then sha256sum "$f"; else shasum -a 256 "$f"; fi
done) > "$stage/MANIFEST.sha256"

archive="$out/oscraft-$version.tar.gz"
tar -C "$stage" -czf "$archive" .
echo "package built archive=$archive version=$version members=$(wc -l < "$stage/MANIFEST.sha256")"
