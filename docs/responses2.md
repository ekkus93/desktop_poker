# Questions on the desktop code review fix docs

These are the remaining clarification questions after reviewing:

1. **Cleanup-pass precedence vs Android truth**
   - `DESKTOP_CODE_REVIEW_FIX_SPEC.md` says it is canonical for this cleanup pass.
   - Should that be interpreted as overriding only older desktop cleanup docs, while still keeping Android code/tests and `docs/ANDROID_DESKTOP_COMPAT_ANSWERS.md` as the source of truth for compatibility-sensitive runtime behavior?

2. **How literally should `DESKTOP_CODE_REVIEW_FIX_TODO.md` be treated?**
   - The file says “Do exactly what is written.”
   - Should this be treated as an ordered implementation checklist, including the audit/discovery work it lists, rather than as a literal script that skips normal code-level verification and judgment?

3. **Can the remaining claimed defects be tied to exact code paths?**
   - The review docs assert:
     - incomplete client public-event consumption
     - misleading event-feed UI
     - stale synthetic invite-generation paths
   - Please point to the exact files/functions/routes/commands that justify each of those claims so the cleanup pass can target the right production code without guessing.

## Already resolved during review

- Ready Room direction: **remove it from production flow**. This matches `docs/DESKTOP_SPECS.md`, which says ready-state behavior is merged into the Tournament Lobby rather than exposed as a separate player-facing Ready Room.
