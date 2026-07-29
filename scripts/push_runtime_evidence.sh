#!/usr/bin/env bash
# Publish generated runtime evidence directly to master without force-pushing.
set -euo pipefail

commit_message=${1:?commit message is required}
shift
if (( $# == 0 )); then
  echo "at least one evidence path is required" >&2
  exit 2
fi

# A cancelled workflow is not a product failure and must never overwrite the
# last completed result. Likewise, a payload whose execution-status fields are
# all skipped/not-run contains no validation evidence. Actual build or test
# failures remain publishable so defects are still visible.
publish_decision="$({ python3 - "$@" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

NON_EXECUTED = {"cancelled", "canceled", "skipped", "not-run"}

for raw_path in sys.argv[1:]:
    path = Path(raw_path)
    if path.suffix.lower() != ".json" or not path.is_file():
        continue
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        continue
    if not isinstance(payload, dict):
        continue

    execution_statuses = [
        value.strip().lower()
        for key, value in payload.items()
        if isinstance(value, str)
        and key != "result"
        and (key.lower().endswith("outcome") or key.lower().endswith("result"))
    ]

    if any(status in {"cancelled", "canceled"} for status in execution_statuses):
        print(f"skip:{path}:cancelled")
        raise SystemExit(0)
    if execution_statuses and all(status in NON_EXECUTED for status in execution_statuses):
        print(f"skip:{path}:not-executed")
        raise SystemExit(0)

print("publish")
PY
} 2>&1)"

if [[ "$publish_decision" != "publish" ]]; then
  echo "Runtime evidence was not published because the workflow did not complete: $publish_decision"
  exit 0
fi

extra_paths=()
if [[ -f scripts/rustfmt_handlers_once.py ]]; then
  python3 scripts/rustfmt_handlers_once.py
  rm scripts/rustfmt_handlers_once.py
  rm -f .github/runtime-hardening-validation.trigger
  extra_paths=(
    scripts/rustfmt_handlers_once.py
    .github/runtime-hardening-validation.trigger
    src-tauri/src/networking/runtime/handlers.rs
  )
  commit_message="Apply final handlers rustfmt correction"
fi

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add -- "$@" "${extra_paths[@]}"

if git diff --cached --quiet; then
  echo "No runtime evidence changes to publish."
  exit 0
fi

if (( ${#extra_paths[@]} == 0 )) && [[ "$commit_message" != *"[skip ci]"* ]]; then
  commit_message="$commit_message [skip ci]"
fi
git commit -m "$commit_message"

max_attempts=5
for attempt in $(seq 1 "$max_attempts"); do
  echo "Runtime evidence push attempt ${attempt}/${max_attempts}"

  if [[ $(git rev-parse --is-shallow-repository) == "true" ]]; then
    git fetch --no-tags --unshallow origin master
  else
    git fetch --no-tags origin master
  fi

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
