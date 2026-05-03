# Questions for Android/Desktop Compatibility

These are the main implementation questions that need answers from the Android side so the desktop app can match it exactly.

1. **Exact protocol schema**
   - What are the exact message types, envelope fields, payload field names, and payload shapes currently used by Android?
   - Which messages are public events vs snapshots vs private encrypted payloads?
   - Are there existing protocol examples or fixtures we should treat as canonical?

2. **Canonical serialization for signatures**
   - What exact algorithm does Android use to produce the signed bytes?
   - Are object keys sorted recursively at every nesting level?
   - How are nulls, numbers, booleans, arrays, and omitted fields handled?
   - Is there an existing test vector set we should match byte-for-byte?

3. **Signature and encryption details**
   - What exact envelope fields are covered by signatures?
   - Is the `signature` field excluded and are any other fields excluded?
   - What exact format is used for encrypted private payload delivery?
   - How are public keys encoded in payloads and envelopes?

4. **Default networking values**
   - What default host port does Android use today?
   - Are there any Android-specific connection assumptions the desktop app must preserve?
   - What exact validation rules does Android apply to `hostAddress` and `hostPort` in the join payload?

5. **Join payload contract**
   - What is the exact serialized format used when Android generates or consumes the direct join payload?
   - Is it raw JSON, base64-wrapped JSON, or another encoding?
   - Are there required/optional fields beyond the ones listed in `DESKTOP_SPECS.md`?
   - Are there versioning rules or backward-compatibility behaviors already in use?

6. **Reconnect identity rules**
   - What exact reconnect token format does Android use?
   - When and how is the reconnect token issued, rotated, invalidated, or expired?
   - What exact state is bound to reconnect eligibility: `playerId`, keypair, seat, session epoch, something else?
   - On Android today, what precise conditions cause reconnect to be accepted or rejected?

7. **Sequence and replay protection**
   - What exact counter and sequence rules does Android enforce?
   - Are counters tracked per sender, per connection, or per session?
   - What exact events advance the authoritative host sequence?
   - How does Android distinguish duplicate, stale, and out-of-order messages?

8. **Tournament constants and rule truth**
   - Are the blind levels and durations in `DESKTOP_SPECS.md` exactly what Android currently uses?
   - Are starting stack presets, timer presets, or seat-capacity rules defined elsewhere in Android code?
   - Are there any known behavior differences between Android’s actual poker engine and the written spec, especially around short all-in, timeout resolution, and showdown behavior?

9. **Snapshots and projections**
   - What exact fields are present in Android snapshots?
   - What exact information is included in public state, per-player private state, and observer state?
   - Are there existing rules or tests that prove hidden information never leaks into the wrong projection?

10. **Interop readiness**
   - Which parts of Android’s current implementation are stable enough to treat as canonical for desktop compatibility?
   - Are there known protocol mismatches, temporary hacks, or in-progress changes the desktop implementation should account for?
   - Is there an existing Android test matrix or fixtures we can reuse for desktop interop tests?
