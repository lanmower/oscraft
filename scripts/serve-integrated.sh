#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
port=${INTEGRATED_PORT:-8842}
site=${INTEGRATED_SITE:-$root/.dev-site-integrated}
pid_file=${INTEGRATED_PID_FILE:-$root/.dev-site-integrated.pid}
log_file=${INTEGRATED_LOG_FILE:-$root/.dev-site-integrated.log}
check=0
stop=0
for arg in "$@"; do
  case "$arg" in
    --check) check=1 ;;
    --stop) stop=1 ;;
    --port=*) port=${arg#--port=} ;;
    --site=*) site=${arg#--site=} ;;
    --pid-file=*) pid_file=${arg#--pid-file=} ;;
    *) echo "usage: scripts/serve-integrated.sh [--check] [--stop] [--port=<port>] [--site=<dir>] [--pid-file=<file>]" >&2; exit 2 ;;
  esac
done
if ! [[ "$port" =~ ^[0-9]+$ ]]; then echo "port must be numeric" >&2; exit 2; fi
for reserved in 8831 8899 8916; do
  if [ "$port" = "$reserved" ]; then echo "integrated port $port is reserved for another server" >&2; exit 2; fi
done

pid_running() {
  [ -f "$pid_file" ] || return 1
  local pid
  pid=$(cat "$pid_file" 2>/dev/null || true)
  [[ "$pid" =~ ^[0-9]+$ ]] || return 1
  kill -0 "$pid" 2>/dev/null || return 1
  printf '%s' "$pid"
}

if [ "$stop" = 1 ]; then
  if pid=$(pid_running); then
    kill -TERM -- "-$pid" 2>/dev/null || kill -TERM "$pid" 2>/dev/null || true
    for _ in 1 2 3 4 5 6 7 8 9 10; do kill -0 "$pid" 2>/dev/null || break; sleep 0.2; done
    kill -KILL -- "-$pid" 2>/dev/null || true
    echo "integrated stop: pid=$pid stopped"
  else
    echo "integrated stop: no owned PID running"
  fi
  rm -f "$pid_file"
  exit 0
fi

if [ "$check" = 1 ]; then
  echo "integrated check: running bounded contract/runtime validation without a server"
  SKIP_PACKS=1 WASM=${WASM:-0} bash "$root/scripts/dev/stage.sh" "$site" >/dev/null
  node "$root/scripts/dev/cross-contract-autoheal.mjs" --once
  node "$root/scripts/dev/watch-cross-runtime.mjs" --once
  node "$root/scripts/validate-runtime-evidence-status.mjs"
  echo "integrated check ok: no server started, no PID file written"
  exit 0
fi

if pid=$(pid_running); then
  echo "integrated already running: pid=$pid port=$port pidFile=$pid_file" >&2
  exit 1
fi

echo "integrated cold start: autoheal then cross-runtime validation"
node "$root/scripts/dev/cross-contract-autoheal.mjs" --once
node "$root/scripts/dev/watch-cross-runtime.mjs" --once
node "$root/scripts/validate-runtime-evidence-status.mjs"

SKIP_PACKS=1 WASM=${WASM:-0} bash "$root/scripts/dev/stage.sh" "$site" >&2
node "$root/scripts/dev/serve.mjs" "$site" --port="$port" --staged-only >"$log_file" 2>&1 &
server_pid=$!
echo "$server_pid" > "$pid_file"
cleanup() {
  if kill -0 "$server_pid" 2>/dev/null; then kill -TERM "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true; fi
  if [ -f "$pid_file" ] && [ "$(cat "$pid_file" 2>/dev/null || true)" = "$server_pid" ]; then rm -f "$pid_file"; fi
}
trap cleanup EXIT INT TERM

base="http://127.0.0.1:$port"
for _ in $(seq 1 100); do
  if curl -fsS "$base/scenarios/moba-acronyms/manifest.json" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
for rel in \
  "/" \
  "/js/client/play.html" \
  "/js/client/play.html?package=scenarios/bota" \
  "/js/client/play.html?package=scenarios/moba" \
  "/scenarios/moba-acronyms/manifest.json" \
  "/scenarios/bota/manifest.json" \
  "/scenarios/moba/manifest.json"; do
  curl -fsS "$base$rel" >/dev/null
done

node - <<'NODE'
const fs=require('fs');
const root=process.cwd();
const status=JSON.parse(fs.readFileSync(root+'/scenarios/bota/runtime-evidence-status.json','utf8'));
const evidence=JSON.parse(fs.readFileSync(root+'/scenarios/bota/runtime-evidence.json','utf8'));
const checks=status.checks;
console.log('INTEGRATED OK');
console.log(`local URL: ${process.env.INTEGRATED_PUBLIC_BASE || `http://127.0.0.1:${process.env.INTEGRATED_PORT || 8842}`}/js/client/play.html`);
console.log(`snapshotHash: ${status.snapshotHash}`);
console.log(`identifier regimes: default ${checks.identifierRegimes.default.pack}/${checks.identifierRegimes.default.activeIds} acronyms; bota ${checks.identifierRegimes.bota.pack}/${checks.identifierRegimes.bota.activeIds} fullnames; historical ${checks.identifierRegimes.historical.pack}`);
console.log(`browser/WebGL: uSkinned=${checks.browser.uSkinned} jointFloats=${checks.browser.jointFloats} models=${checks.browser.models} heroes=${checks.browser.heroes} clips=${checks.browser.clips} motion=${checks.browser.motion}`);
for (const row of status.extractionSummaries) console.log(`extraction ${row.peer}: present=${row.present} classes=${row.classes ?? '?'} complete=${row.complete ?? '?'} blocked=${row.blocked ?? '?'} informational=true`);
console.log(`ok: ${status.ok === true && evidence.ok === true}`);
NODE
printf 'INTEGRATED PID: %s PORT: %s PIDFILE: %s\n' "$server_pid" "$port" "$pid_file"
wait "$server_pid"
