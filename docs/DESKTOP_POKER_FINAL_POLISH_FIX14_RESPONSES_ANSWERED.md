# Responses — Fix 14: Final Validation Ledger and Cleanup Comments
# Spec: docs/DESKTOP_POKER_FINAL_POLISH_FIX14_SPEC.md
# TODO: docs/DESKTOP_POKER_FINAL_POLISH_FIX14_TODO.md

1. Q: Fix 13 ledger history (P0.1): P0.1 chooses wording based on whether `cargo test -p poker-core` was actually run in Fix 13. The implementor will check the `memory.md` Fix 13 entry and treat absence as "was not run." Is that the right interpretation, or do you have knowledge that it was run but just not recorded? (If the implementor runs the full suite during Fix 14, this is moot — they record what they ran.)
   A:

2. Q: P0.3 regression handling: If the reconnect command-stream install pattern audit (P0.3) finds that Fix 13 behavior has somehow regressed (e.g., a subsequent commit weakened it), should the implementor fix it inline as part of Fix 14, or surface it as a blocker and stop? The spec says "preserve," which implies fix-inline, but it's worth confirming since Fix 14 is supposed to be comment/ledger-only.
   A: Fix it inline as part of Fix 14 if the regression is directly in the reconnect command-stream install behavior and the patch remains small. The “final polish” scope does not mean leaving a known silent reconnect failure in place. The implementor should restore the Fix 13 invariant: clone failure or command-connection lock failure emits `SafeError`, does not emit the accepted reconnect snapshot, and stops/breaks the runtime path. If the audit exposes a broader architectural regression that cannot be fixed with a small local patch, then stop and surface it as a blocker with exact file/line details and the failing audit command.

---
Fill in the A: lines above and share back when ready.
