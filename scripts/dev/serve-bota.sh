#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
port=${BOTA_PORT:-0}
out=${BOTA_SITE:-$root/.dev-site-bota}
for arg in "$@"; do
  case "$arg" in
    --port=*) port=${arg#--port=} ;;
    *) echo "usage: BOTA_PORT=<port> scripts/dev/serve-bota.sh [--port=<port>]" >&2; exit 2 ;;
  esac
done

if ! [[ "$port" =~ ^[0-9]+$ ]]; then
  echo "port must be numeric" >&2
  exit 2
fi

SKIP_PACKS=1 WASM=${WASM:-0} bash "$root/scripts/dev/stage.sh" "$out" >&2
if [ "$port" = 0 ]; then
  port=$(node -e 'const s=require("node:net").createServer().listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close()})')
fi
url="http://127.0.0.1:$port/js/client/play.html?package=scenarios/bota"
printf 'BOTA URL: %s\n' "$url"
printf 'BOTA server: pid=%s port=%s site=%s\n' "$$" "$port" "$out"

cleanup() {
  if [ -n "${server_pid:-}" ]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM

node "$root/scripts/dev/serve.mjs" "$out" --port="$port" --staged-only &
server_pid=$!
printf 'BOTA process: node scripts/dev/serve.mjs --port=%s --staged-only (pid=%s)\n' "$port" "$server_pid"

wait "$server_pid"
