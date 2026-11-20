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
top_out=$(top -b -n 2 -d 1)
heavy=""
while read -r pid cpu; do
  [ -z "$pid" ] && continue
  args=$(ps -o args= -p "$pid" 2>/dev/null | cut -c1-120)
  case "$args" in *--handshake*) continue ;; esac
  heavy="$heavy$pid($cpu% $args) "
done < <(echo "$top_out" | awk '/PID USER/{f++} f==2 && $9+0>=20 {print $1, $9}')
echo "heavy (top current CPU at or above 20%, gm-core --handshake workers excluded by full ps args): [$heavy]"
if [ -n "$heavy" ]; then
  echo "HEAVY JOB RUNNING: no witness started: $heavy"
  exit 4
fi
s=$(date +%s.%N)
"$@"
rc=$?
e=$(date +%s.%N)
echo "wall=$(awk -v a=$s -v b=$e 'BEGIN{printf "%.2f", b-a}')s rc=$rc"
exit $rc
