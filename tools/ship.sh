#!/usr/bin/env bash
set -euo pipefail
if [ "$#" -lt 2 ]; then
  echo "usage: tools/ship.sh <commit-message-file> <path...>" >&2
  exit 2
fi
message_file="$1"
shift
git add -- "$@"
git commit -F "$message_file" -- "$@"
for attempt in 1 2 3; do
  git fetch origin main
  git rebase origin/main
  if git push origin main; then
    exit 0
  fi
done
exit 1
