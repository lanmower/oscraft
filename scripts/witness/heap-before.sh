#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:?usage: heap-before.sh <empty-dir>}
mkdir -p "$out/js/client"
git -C "$root" show HEAD:js/client/play.js > "$out/js/client/play.js"
git -C "$root" show HEAD:js/client/hud.js > "$out/js/client/hud.js"
cd "$out"
patch -R -p1 --silent < "$root/scripts/witness/heap-before-j.patch"
! grep -q savedDrawCounts "$out/js/client/play.js"
echo "baseline written to $out (J1, J6, J7, J12, J14 reversed)"
