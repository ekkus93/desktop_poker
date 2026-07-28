# Desktop DTO Contract Policy

Desktop Poker currently uses an **explicit shared-fixture contract** for Rust-to-TypeScript desktop DTOs.

The authoritative serialized field names come from the Rust structs and their Serde attributes. The committed fixture at `src/fixtures/desktop-contract.json` records the required top-level keys for the DTO families used most heavily by the React shell:

- `DesktopBootstrapState`
- `HostSessionStatus`
- `ClientSessionStatus`
- `TableViewSnapshot`
- `DebugInspectorState`
- `NpcProfileListResult`
- LLM provider settings and configuration

`src-tauri/tests/desktop_contract.rs` serializes the real Rust types and compares their keys with the fixture. `src/api/desktop.contract.test.ts` constructs values checked by the TypeScript compiler and compares their keys with the same fixture. A field addition, removal, or Serde rename therefore requires an intentional fixture and frontend contract update.

Generated bindings are not being introduced during runtime hardening. `specta`/`tauri-specta` or `ts-rs` may be evaluated in a separate focused architecture change after the v1 protocol and desktop command surfaces stabilize. Until then, manually mirrored DTOs must remain covered by the shared fixture tests.
