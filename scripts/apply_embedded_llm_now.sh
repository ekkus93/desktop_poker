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

# Remove every one-shot bootstrap path from the generated repository.
rm -f scripts/.embedded_patch.part.*
rm -f scripts/apply_embedded_llm_now.sh
rm -f .github/workflows/apply-embedded-llm.yml
rm -f .github/workflows/diagnose-embedded-apply.yml
rm -f .github/workflows/apply-embedded-llm-now.yml
rm -f docs/runtime-validation/embedded-apply.log

python3 - <<'PY'
from pathlib import Path


def remove_job(path: str, job: str, next_job: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    marker = f"\n  {job}:\n"
    if marker in text:
        start = text.index(marker)
        end = text.index(f"\n  {next_job}:\n", start)
        text = text[:start] + text[end:]
    p.write_text(text, encoding="utf-8")


remove_job(".github/workflows/ci.yml", "apply-embedded-local-llm", "verify")
remove_job(
    ".github/workflows/runtime-validation.yml",
    "apply-embedded-local-llm",
    "release-runtime",
)
ci = Path(".github/workflows/ci.yml")
text = ci.read_text(encoding="utf-8").replace(
    "  cancel-in-progress: false\n", "  cancel-in-progress: true\n", 1
)
ci.write_text(text, encoding="utf-8")
PY

# Generate reproducible formatting and dependency metadata before publication.
cargo fmt --all
npm ci
npm run format
cargo generate-lockfile

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add -A
git commit -m "Add embedded GGUF NPC inference"
publish_with_retry
