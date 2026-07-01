# Fix 8 Responses — Lock Shared-Core Baseline and CI Validation

Fill in each `A:` line, then share this file back (or paste your answers).

---

1. Q: **P0.1 release job cache**: Should the Rust cache in the `release` CI job also be updated from `workspaces: src-tauri -> target` to `workspaces: . -> target` for consistency, or left as-is since the release job has no standalone Rust validation steps?
   A: Update the release job cache to `workspaces: . -> target` as well. Even if the release job does not run standalone Rust validation, it builds/packages the Tauri app in a workspace that now includes `crates/poker-core`. Keeping one job on the old `src-tauri -> target` cache shape is stale documentation-by-config and increases the chance of confusing future CI/debug work. Use the workspace-root cache consistently anywhere Rust/Tauri build artifacts are involved. Do not change unrelated release semantics; this is just the cache workspace path.

2. Q: **P0.2 duplicate section**: `.github/copilot-instructions.md` has a duplicated "Memory file" section (appears twice). Should it be cleaned up (removing the duplicate) as part of P0.2?
   A: Yes. Clean it up as part of P0.2. The point of P0.2 is to prevent future coding agents from reading stale or contradictory instructions. A duplicated section is exactly the kind of low-grade instruction drift that causes later confusion. Keep one canonical "Memory file" section, preserve the current correct content, and remove the duplicate. Do not rewrite the whole file beyond making it workspace-aware and de-duplicated.

3. Q: **P0.3 test matchers**: The suggested test uses `.toHaveTextContent()`, which is a jest-dom matcher not available in this project's Vitest setup (same class of error as `.toBeInTheDocument()` in Fix 7). Should the P0.3 test use `element.textContent` / `.toBeTruthy()` patterns consistent with existing tests — or should jest-dom be added to the Vitest setup first?
   A: Use the existing project style: `element.textContent`, `.toBeTruthy()`, `.not.toBeNull()`, and normal Vitest matchers. Do not add jest-dom just for this test. Adding jest-dom is a broader test-environment change and is unnecessary for this small assertion. The goal is to keep Fix 8 narrow and avoid introducing new test setup dependencies while fixing the missing health counter rendering.

4. Q: **P0.4 CI enforcement**: Should `npm run format:check` be added to the CI `verify` job, or is formatting considered a local-only convention (enforced by `npm run format` before commit, not by CI)?
   A: Add `npm run format:check` to the CI `verify` job. The previous review found `npm run format:check` failing while lint/build/tests passed, which means local-only formatting is not enough for this project. CI should fail on formatting drift so future agents cannot claim validation is clean while the formatter disagrees. Keep `npm run format` as the local fixing command, but CI should run the non-mutating check.

5. Q: **P0.5 mutation approach**: `HostServer::replace_authoritative_state` already exists as a public method and can be used to inject corrupted hole-card state in the test, making the suggested new `#[cfg(test)]` helper unnecessary. Should the test use `replace_authoritative_state` directly, or is there a reason to prefer the new mutation helper?
   A: Use `HostServer::replace_authoritative_state` directly if it can set up the corrupted state cleanly. Do not add a new `#[cfg(test)]` mutation helper unless `replace_authoritative_state` proves insufficient. The purpose of P0.5 is to get a real action-path regression test, not to add extra test-only APIs. Using the existing authoritative-state replacement method is simpler and avoids expanding the host API surface.
