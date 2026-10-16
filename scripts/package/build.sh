#!/usr/bin/env bash
# Builds one installable archive: oscraftd, oscraft-lsp, the data directory and the browser site.
# Usage: scripts/package/build.sh <out-dir>
# Output: <out-dir>/oscraft-<version>.tar.gz, members at its root (bin/ data/ site/ VERSION MANIFEST.sha256).
set -euo pipefail
out=${1:?usage: build.sh <out-dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
rust="$root/rust"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust/crates/server/Cargo.toml" | head -n 1)
stage="$out/stage"
rm -rf "$out"
mkdir -p "$stage/bin" "$stage/data" "$stage/site"

cd "$rust"
cargo build --release -q -p server --bin oscraftd
cargo build --release -q -p oscraft-lsp
cp target/release/oscraftd target/release/oscraft-lsp "$stage/bin/"
cp data/*.star "$stage/data/"
cp -R crates/web/site/. "$stage/site/"
cargo run -q -p mapexport -- "$stage/site/map.bin"
wasm-pack build crates/web --target web --out-dir "$stage/site/pkg" > /dev/null
cd "$root"

printf '%s\n' "$version" > "$stage/VERSION"
(cd "$stage" && find . -type f ! -name MANIFEST.sha256 | sed 's|^\./||' | LC_ALL=C sort | while read -r f; do
  if command -v sha256sum > /dev/null; then sha256sum "$f"; else shasum -a 256 "$f"; fi
done) > "$stage/MANIFEST.sha256"

archive="$out/oscraft-$version.tar.gz"
tar -C "$stage" -czf "$archive" .
echo "package built archive=$archive version=$version members=$(wc -l < "$stage/MANIFEST.sha256")"
