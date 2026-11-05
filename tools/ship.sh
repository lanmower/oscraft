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
  if git merge-base --is-ancestor origin/main HEAD; then
    if git push origin main; then
      exit 0
    fi
  else
    git reset --soft origin/main
    git commit -F "$message_file" -- "$@"
    if git push origin main; then
      exit 0
    fi
  fi
done
exit 1
