# Responses — Fix 14: Final Validation Ledger and Cleanup Comments
# Spec: docs/DESKTOP_POKER_FINAL_POLISH_FIX14_SPEC.md
# TODO: docs/DESKTOP_POKER_FINAL_POLISH_FIX14_TODO.md

1. Q: Fix 13 ledger history (P0.1): P0.1 chooses wording based on whether `cargo test -p poker-core` was actually run in Fix 13. The implementor will check the `memory.md` Fix 13 entry and treat absence as "was not run." Is that the right interpretation, or do you have knowledge that it was run but just not recorded? (If the implementor runs the full suite during Fix 14, this is moot — they record what they ran.)
   A:

2. Q: P0.3 regression handling: If the reconnect command-stream install pattern audit (P0.3) finds that Fix 13 behavior has somehow regressed (e.g., a subsequent commit weakened it), should the implementor fix it inline as part of Fix 14, or surface it as a blocker and stop? The spec says "preserve," which implies fix-inline, but it's worth confirming since Fix 14 is supposed to be comment/ledger-only.
   A:

---
Fill in the A: lines above and share back when ready.
