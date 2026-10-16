#!/usr/bin/env bash
# Unpacks an archive into an empty directory, checks every member against MANIFEST.sha256,
# then runs oscraftd from that directory and drives oscraft-lsp over stdio.
# Usage: scripts/package/install-check.sh <archive.tar.gz> <empty-dir>
set -euo pipefail
archive=${1:?usage: install-check.sh <archive> <empty-dir>}
dest=${2:?usage: install-check.sh <archive> <empty-dir>}
addr=${OSCRAFT_CHECK_ADDR:-127.0.0.1:18477}
fail() { echo "FAIL: $*" >&2; exit 1; }

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
help=$(node js/host/server.js --help) || fail "node js/host/server.js --help exited nonzero in the unpacked archive"
grep -q '^usage: node js/host/server.js --wasm' <<< "$help" || fail "node js/host/server.js --help printed no usage line"
[ -s js/client/play.js ] || fail "js/client/play.js is missing or empty"
[ -f js/client/play.html ] || fail "js/client/play.html is missing"
grep -q 'src="play.js"' js/client/play.html || fail "js/client/play.html does not load play.js"
specs=$(for f in js/client/play.js js/client/delta.js; do node -e 'const s=require("fs").readFileSync(process.argv[1],"utf8");const re=/(?:from|import\()\s*["\x27](\.\.?\/[^"\x27]+)["\x27]/g;let m;while((m=re.exec(s)))console.log(m[1]);' "$f"; done | sort -u)
refs=$(node -e 'const s=require("fs").readFileSync("js/client/play.html","utf8");const re=/data-(?:pkg|map)="([^"]+)"/g;const out=[];let m;while((m=re.exec(s)))out.push(m[1]);process.stdout.write(out.join("\n"))')
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

server_pid=
cleanup() { [ -n "$server_pid" ] && kill "$server_pid" 2>/dev/null || true; }
trap cleanup EXIT
log="$dest/server.log"
./bin/oscraftd --addr "$addr" --room ci --data "$dest/data" --site "$dest/site" > "$log" 2>&1 &
server_pid=$!
for _ in $(seq 1 120); do curl -sf "http://$addr/state" > /dev/null 2>&1 && break; sleep 0.5; done
curl -sf "http://$addr/state" | jq -e '.units | numbers' > /dev/null || { cat "$log" >&2; fail "oscraftd did not serve /state from the unpacked archive"; }
curl -sf "http://$addr/site/" > /dev/null || fail "oscraftd did not serve /site/ from the unpacked archive"

frame() { printf 'Content-Length: %d\r\n\r\n%s' "${#1}" "$1"; }
lsp_out=$({
  frame '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}'
  frame '{"jsonrpc":"2.0","id":2,"method":"shutdown"}'
  frame '{"jsonrpc":"2.0","method":"exit"}'
} | timeout 30 ./bin/oscraft-lsp) || fail "oscraft-lsp did not start and exit cleanly"
printf '%s' "$lsp_out" | grep -q '"capabilities"' || fail "oscraft-lsp initialize reply has no capabilities"

echo "package install ok members=$members sha256_checked=$checked"
echo "client imports ok=$imports"
echo "package install ok members=$members sha256_checked=$checked host_help=ok client=ok"
