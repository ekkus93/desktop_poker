# Fix 8 Responses — Lock Shared-Core Baseline and CI Validation

Fill in each `A:` line, then share this file back (or paste your answers).

---

1. Q: **P0.1 release job cache**: Should the Rust cache in the `release` CI job also be updated from `workspaces: src-tauri -> target` to `workspaces: . -> target` for consistency, or left as-is since the release job has no standalone Rust validation steps?
   A:

2. Q: **P0.2 duplicate section**: `.github/copilot-instructions.md` has a duplicated "Memory file" section (appears twice). Should it be cleaned up (removing the duplicate) as part of P0.2?
   A:

3. Q: **P0.3 test matchers**: The suggested test uses `.toHaveTextContent()`, which is a jest-dom matcher not available in this project's Vitest setup (same class of error as `.toBeInTheDocument()` in Fix 7). Should the P0.3 test use `element.textContent` / `.toBeTruthy()` patterns consistent with existing tests — or should jest-dom be added to the Vitest setup first?
   A:

4. Q: **P0.4 CI enforcement**: Should `npm run format:check` be added to the CI `verify` job, or is formatting considered a local-only convention (enforced by `npm run format` before commit, not by CI)?
   A:

5. Q: **P0.5 mutation approach**: `HostServer::replace_authoritative_state` already exists as a public method and can be used to inject corrupted hole-card state in the test, making the suggested new `#[cfg(test)]` helper unnecessary. Should the test use `replace_authoritative_state` directly, or is there a reason to prefer the new mutation helper?
   A:
