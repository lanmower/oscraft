#!/bin/bash
set -u
reading=$(vmstat 1 2 | tail -1)
si=$(echo "$reading" | awk '{print $7}')
so=$(echo "$reading" | awk '{print $8}')
echo "gate second sample: si=$si so=$so KB/s"
if ! [[ "$si" =~ ^[0-9]+$ && "$so" =~ ^[0-9]+$ ]] || [ "$si" -ge 1000 ] || [ "$so" -ge 1000 ]; then
  echo "GATE CLOSED: no witness started"
  exit 3
fi
heavy=$(node "$(dirname "$0")/heavy-jobs.cjs" 2>&1); hrc=$?
echo "$heavy"
if [ $hrc -ne 0 ]; then
  echo "HEAVY JOB RUNNING: no witness started (heavy-jobs.cjs rc=$hrc)"
  exit 4
fi
s=$(date +%s.%N)
"$@"
rc=$?
e=$(date +%s.%N)
echo "wall=$(awk -v a=$s -v b=$e 'BEGIN{printf "%.2f", b-a}')s rc=$rc"
exit $rc
