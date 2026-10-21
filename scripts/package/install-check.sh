#!/usr/bin/env bash
# Unpacks an archive into an empty directory and checks it without starting a server: every member against
# MANIFEST.sha256, the client's imports and page references resolve inside the archive, the moba and duel-routes map
# images and the wasm package are present and non-empty, and bin/oscraft-lsp starts, answers initialize and exits.
# Usage: scripts/package/install-check.sh <archive.tar.gz> <empty-dir>
set -euo pipefail
archive=$(realpath "${1:?usage: install-check.sh <archive> <empty-dir>}")
dest=${2:?usage: install-check.sh <archive> <empty-dir>}
fail() { echo "FAIL: $*" >&2; exit 1; }
exe=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) exe=.exe ;; esac

root_src=$(cd "$(dirname "$0")/../.." && pwd)/js/client
mkdir -p "$dest"
[ -z "$(ls -A "$dest")" ] || fail "destination $dest is not empty"
tar -C "$dest" -xzf "$archive"
cd "$dest"
[ -f MANIFEST.sha256 ] || fail "archive has no MANIFEST.sha256"

members=0
checked=0
while read -r want path; do
  members=$((members + 1))
  [ -f "$path" ] || fail "manifest member missing from archive: $path"
  if command -v sha256sum > /dev/null; then got=$(sha256sum "$path" | cut -d' ' -f1)
  else got=$(shasum -a 256 "$path" | cut -d' ' -f1); fi
  [ "$got" = "$want" ] || fail "sha256 mismatch for $path: manifest=$want actual=$got"
  checked=$((checked + 1))
done < MANIFEST.sha256
extra=$(find . -type f ! -name MANIFEST.sha256 | wc -l)
[ "$extra" -eq "$members" ] || fail "archive holds $extra files but manifest lists $members"

[ -s "bin/oscraft-lsp$exe" ] || fail "bin/oscraft-lsp$exe is missing or empty"
[ -s js/client/play.js ] || fail "js/client/play.js is missing or empty"
for f in $(cd "$root_src" 2>/dev/null && ls *.js *.html 2>/dev/null) editor.html editor.js play.html play.js rooms.js ice.js delta.js wire/frame.js glb.js pkg/web.js pkg/web_bg.wasm; do
  [ -s "js/client/$f" ] || fail "js/client/$f is missing or empty"
done
[ -s scenarios/moba/moba.osdt ] || fail "scenarios/moba/moba.osdt (the moba map image) is missing or empty"
[ -s scenarios/duel-routes/duel-routes.osdt ] || fail "scenarios/duel-routes/duel-routes.osdt is missing or empty"
[ -n "$(ls mods/*.lua 2>/dev/null)" ] || fail "mods/ is missing or empty"
[ -d scenarios/moba/assets/textures ] || fail "scenarios/moba/assets/textures is missing"
[ -d scenarios/moba/assets/models ] && [ -n "$(ls -A scenarios/moba/assets/models)" ] || fail "scenarios/moba/assets/models is missing or empty"
grep -q 'src="play.js"' js/client/play.html || fail "js/client/play.html does not load play.js"
specs=$(grep -ho -E "[\"'][.][.]?/[^\"']+[\"']" js/client/play.js js/client/delta.js | tr -d "\"'" | sort -u)
refs=$(grep -ho -E 'data-(pkg|map)="[^"]+"' js/client/play.html | sed -E 's/.*="(.*)"/\1/')
imports=0
while IFS= read -r spec; do
  [ -n "$spec" ] || continue
  imports=$((imports + 1))
  [ -f "js/client/$spec" ] || fail "client module imports $spec, which is not in the archive"
done <<< "$specs"
while IFS= read -r ref; do
  [ -n "$ref" ] || continue
  imports=$((imports + 1))
  [ -f "js/client/$ref" ] || fail "js/client/play.html references $ref, which is not in the archive"
done <<< "$refs"
[ "$imports" -ge 1 ] || fail "no relative import or page reference was found to check"

# The language server reads frames on stdin and exits on the exit notification; timeout is a backstop where it exists.
frame() { printf 'Content-Length: %d\r\n\r\n%s' "${#1}" "$1"; }
lsp() { if command -v timeout > /dev/null; then timeout 30 "$@"; else "$@"; fi; }
lsp_out=$({
  frame '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
  frame '{"jsonrpc":"2.0","id":2,"method":"shutdown"}'
  frame '{"jsonrpc":"2.0","method":"exit"}'
} | lsp "./bin/oscraft-lsp$exe") || fail "oscraft-lsp did not start and exit cleanly"
printf '%s' "$lsp_out" | grep -q '"capabilities"' || fail "oscraft-lsp initialize reply has no capabilities"

echo "client imports ok=$imports"
echo "package install ok members=$members sha256_checked=$checked client=ok pkg=ok map=ok lsp=ok"
