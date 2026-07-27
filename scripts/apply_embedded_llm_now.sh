#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
log_tmp="$(mktemp)"
trap 'rm -f "$log_tmp"' EXIT

publish_with_retry() {
  for attempt in 1 2 3 4 5 6 7 8; do
    if git pull --rebase origin master && git push origin HEAD:master; then
      return 0
    fi
    sleep 3
  done
  return 1
}

fail_with_diagnostic() {
  local status="$1"
  git reset --hard HEAD
  rm -f src-tauri/src/npc/embedded_llm.rs
  mkdir -p docs/runtime-validation
  cp "$log_tmp" docs/runtime-validation/embedded-apply.log
  cat docs/runtime-validation/embedded-apply.log
  git config user.name "github-actions[bot]"
  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  git add docs/runtime-validation/embedded-apply.log
  if git diff --cached --quiet; then
    printf '\ndiagnostic output was unchanged\n' >> docs/runtime-validation/embedded-apply.log
    git add docs/runtime-validation/embedded-apply.log
  fi
  git commit -m "docs: record embedded LLM transformation failure"
  publish_with_retry
  exit "$status"
}

cat scripts/.embedded_patch.part.* > /tmp/apply_embedded_llm_changes.py
if ! python3 -m py_compile /tmp/apply_embedded_llm_changes.py >"$log_tmp" 2>&1; then
  fail_with_diagnostic 1
fi
if ! python3 /tmp/apply_embedded_llm_changes.py >"$log_tmp" 2>&1; then
  fail_with_diagnostic 1
fi

# GitHub's Actions token may push source but cannot create or update workflow
# files. Restore workflows here; those changes are applied afterward through
# the connected GitHub write API.
git checkout HEAD -- .github/workflows

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add -A
git commit -m "Add embedded GGUF NPC inference"
publish_with_retry
