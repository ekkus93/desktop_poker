# Android Architecture

The Android app will be a native Kotlin/Compose app. It will not use Tauri Mobile.

## Boundary

Kotlin owns:
- Compose UI
- ViewModel/Repository state flow
- Android lifecycle
- permissions
- storage integration
- networking/session transport (TCP, NSD, QR join)

Rust `poker-core` owns:
- poker rules
- tournament state transitions
- legal action validation
- dealing/shuffling
- showdown/settlement
- public/private projection
- deterministic state serialization

## Networking

Networking is platform/session adapter code. It is not part of `poker-core`.

Incoming network messages should be validated and routed by Kotlin, then converted into `EngineCommand`s. The core returns events and state snapshots. Kotlin sends resulting updates over Android-owned networking and updates Compose state.

`poker-core` has no socket, Tauri, LLM, keychain, or platform dependencies. Verified by `cargo tree -p poker-core`.

## Binding direction

Preferred future binding: UniFFI. Hand-written JNI is acceptable only if UniFFI cannot represent the needed API cleanly.

A future `crates/poker-android-ffi/` crate would:
- depend on `poker-core`
- expose UniFFI/JNI-safe DTOs
- have no Tauri dependency
- have no Android UI code inside Rust
