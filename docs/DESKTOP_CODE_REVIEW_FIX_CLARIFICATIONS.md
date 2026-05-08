# DESKTOP_CODE_REVIEW_FIX_CLARIFICATIONS.md

This file answers Copilot's follow-up questions on the desktop code review fix docs.

## 1. Cleanup-pass precedence vs Android truth

`DESKTOP_CODE_REVIEW_FIX_SPEC.md` is canonical only for this desktop cleanup pass.

It overrides older desktop cleanup/review docs, but it does **not** override Android compatibility truth.

For compatibility-sensitive behavior, follow this order:

1. current Android runtime code and tests
2. `docs/ANDROID_DESKTOP_COMPAT_ANSWERS.md`
3. `DESKTOP_SPECS.md`
4. `DESKTOP_CODE_REVIEW_FIX_SPEC.md`
5. `DESKTOP_CODE_REVIEW_FIX_TODO.md`

In plain terms:
- use the cleanup docs to fix desktop implementation defects
- use Android code/tests and compatibility answers to match protocol details

## 2. How literally to treat the TODO

Treat the TODO as an ordered implementation checklist.

Do the audit/discovery tasks. They are required.

Do not treat the TODO as a blind script that skips code-level verification. Copilot must inspect references before deleting code, preserve correct production behavior, and add tests for fixed defects.

## 3. Exact code paths for remaining claimed defects

### 3.1 Incomplete client public-event consumption

Primary file/function:
- `src-tauri/src/app_state/mod.rs`
- `DesktopClientSession::apply_event(...)`

Current issue:
- `ClientRuntimeEvent::PublicEvent { .. }`
- `ClientRuntimeEvent::PrivateHoleCards(_)`
- `ClientRuntimeEvent::ResyncRequested { .. }`

are ignored.

Related emitter:
- `src-tauri/src/networking/runtime.rs`
- `ClientRuntimeEvent::PublicEvent`
- `ClientRuntimeEvent::PrivateHoleCards`
- `ClientRuntimeEvent::ResyncRequested`

Required fix:
- public events must update client session truth
- private card events must update recipient-private state
- resync requests must trigger real resync behavior

### 3.2 Misleading event-feed UI

Frontend files:
- `src/screens/MainTableScreen.tsx`
  - renders `Latest public events`
- `src/screens/HandHistoryScreen.tsx`
  - renders `eventFeed`

Backend files/functions:
- `src-tauri/src/app_state/mod.rs`
  - `DesktopHostSession::table_view(...)`
  - `DesktopClientSession::table_view(...)`
  - both pass `Vec::new()` as the event feed
- `src-tauri/src/commands.rs`
  - fallback table view also uses `event_feed: vec![]`

Required fix:
- either feed real production event data
- or remove/hide the event-feed UI from production

### 3.3 Stale synthetic invite-generation path

Files/functions:
- `src-tauri/src/commands.rs`
  - `create_host_invite(...)`
  - `create_host_invite_inner(...)`
- `src-tauri/src/lib.rs`
  - command registration includes `create_host_invite`
- `src/api/desktop.ts`
  - exports `createHostInvite(...)`

Problem:
- `create_host_invite_inner(...)` builds payload fields from `DesktopBootstrapState`
- it fabricates table ID, host signing key, and join token
- it is not tied to a real live host session

Correct production source:
- `src-tauri/src/app_state/mod.rs`
- `DesktopHostSession::status(...)`
- `HostSessionStatus.invite`
- `self.host_server.encoded_join_payload()`

Required fix:
- production invite sharing must use `HostSessionStatus.invite`
- synthetic invite command must be removed from production or gated/renamed debug-only

## 4. Ready Room direction

Ready Room is removed from production flow.

Targets:
- `src/screens/ReadyRoomScreen.tsx`
- `src/app/AppShell.tsx`
- `src/components/layout/AppFrame.tsx`
- `src/test/appIntegrationFixtures.ts`
- `src/probe/LayoutProbeApp.tsx`

Required fix:
- no production route to `/ready-room`
- readiness behavior belongs in Tournament Lobby
- if Ready Room remains at all, it must be debug/probe-only

