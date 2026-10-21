#!/usr/bin/env bash
# Builds one installable archive with the layout .github/workflows/pages.yml deploys, plus the language server:
# bin/oscraft-lsp, js/client (the page, its wire/ and vendor/ and the wasm package in js/client/pkg), scenarios/moba and
# scenarios/duel-routes with their map images, mods/, VERSION and MANIFEST.sha256.
# Usage: scripts/package/build.sh <out-dir>
# Output: <out-dir>/oscraft-<version>.tar.gz, members at its root. The unpacked tree is left in <out-dir>/stage.
set -euo pipefail
out=${1:?usage: build.sh <out-dir>}
root=$(cd "$(dirname "$0")/../.." && pwd)
rust="$root/rust"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$rust/crates/mod-lsp/Cargo.toml" | head -n 1)
[ -n "$version" ] || { echo "no version in rust/crates/mod-lsp/Cargo.toml" >&2; exit 1; }
# Windows builds name their binaries .exe; the staged names stay the same on every platform.
exe=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) exe=.exe ;; esac
stage="$out/stage"
rm -rf "$out"
mkdir -p "$stage/bin" "$stage/js" "$stage/scenarios"

cd "$rust"
cargo build --release -q -p oscraft-lsp -p mapexport
cp "target/release/oscraft-lsp$exe" "$stage/bin/oscraft-lsp$exe"

cd "$root"
cp -R js/client "$stage/js/client"
find "$stage/js/client" \( -name node_modules -o -name .git \) -prune -exec rm -rf {} +
rm -rf "$stage/js/client/pkg"
for page in play.html editor.html replay.html; do
  [ -f "$stage/js/client/$page" ] || { echo "missing js/client/$page" >&2; exit 1; }
done
cp -R mods "$stage/mods"
for s in moba duel-routes; do cp -R "scenarios/$s" "$stage/scenarios/$s"; done

cd "$rust"
wasm-pack build crates/web --target web --out-dir "$stage/js/client/pkg" > /dev/null
rm -f "$stage/js/client/pkg/.gitignore"

# mapexport reads the staged copy and writes <name>.osdt into it, as pages.yml does for the repo tree.
for s in moba duel-routes; do
  "$rust/target/release/mapexport$exe" "$stage/scenarios/$s" "$stage/scenarios/$s/$s.osdt"
done
mkdir -p "$stage/scenarios/moba/assets/models"
cp "$rust"/crates/web/site/assets/units/*.glb "$stage/scenarios/moba/assets/models/"

printf '%s\n' "$version" > "$stage/VERSION"
(cd "$stage" && find . -type f ! -name MANIFEST.sha256 | sed 's|^\./||' | LC_ALL=C sort | while read -r f; do
  if command -v sha256sum > /dev/null; then sha256sum "$f"; else shasum -a 256 "$f"; fi
done) > "$stage/MANIFEST.sha256"

archive="$out/oscraft-$version.tar.gz"
tar -C "$stage" -czf "$archive" .
echo "package built archive=$archive version=$version members=$(wc -l < "$stage/MANIFEST.sha256")"
