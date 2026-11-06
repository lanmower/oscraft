#!/usr/bin/env bash
set -euo pipefail
out=${1:?usage: build.sh <out-dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
rust="$root/rust"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust/crates/mod-lsp/Cargo.toml" | head -n 1)
[ -n "$version" ] || { echo "no version in rust/crates/mod-lsp/Cargo.toml" >&2; exit 1; }
exe=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) exe=.exe ;; esac
stage="$out/stage"
rm -rf "$out"
mkdir -p "$out"

cd "$rust"
cargo build --release -q -p oscraft-lsp -p mapexport
wasm-pack build crates/web --target web --out-dir crates/web/site/pkg > /dev/null

cd "$root"
for s in moba duel-routes; do
  "$rust/target/release/mapexport$exe" "scenarios/$s" "scenarios/$s/$s.osdt"
done

SKIP_PACKS=1 bash scripts/layout-site.sh "$stage" "$version" "release"
find "$stage/js/client" \( -name node_modules -o -name .git \) -prune -exec rm -rf {} +

mkdir -p "$stage/bin"
cp "$rust/target/release/oscraft-lsp$exe" "$stage/bin/oscraft-lsp$exe"
printf '%s\n' "$version" > "$stage/VERSION"
(cd "$stage" && find . -type f ! -name MANIFEST.sha256 | sed 's|^\./||' | LC_ALL=C sort | while read -r f; do
  if command -v sha256sum > /dev/null; then sha256sum "$f"; else shasum -a 256 "$f"; fi
done) > "$stage/MANIFEST.sha256"

archive="$out/oscraft-$version.tar.gz"
(cd "$stage" && COPYFILE_DISABLE=1 tar -czf "../oscraft-$version.tar.gz" .)
echo "package built archive=$archive version=$version members=$(wc -l < "$stage/MANIFEST.sha256")"
