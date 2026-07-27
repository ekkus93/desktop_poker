#!/usr/bin/env bash
set -euo pipefail

commit_message=${1:?commit message is required}
shift
if (( $# == 0 )); then
  echo "at least one evidence path is required" >&2
  exit 2
fi

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add -- "$@"

if git diff --cached --quiet; then
  echo "No runtime evidence changes to publish."
  exit 0
fi

git commit -m "$commit_message"

max_attempts=5
for attempt in $(seq 1 "$max_attempts"); do
  echo "Runtime evidence push attempt ${attempt}/${max_attempts}"
  git fetch --no-tags origin master
  git rebase origin/master

  if git push origin HEAD:master; then
    echo "Runtime evidence published successfully."
    exit 0
  fi

  if (( attempt == max_attempts )); then
    echo "Unable to publish runtime evidence after ${max_attempts} attempts." >&2
    exit 1
  fi

  sleep $((attempt * 2))
done
