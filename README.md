# Desktop Poker

[![CI](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml)

Desktop Poker is a Linux-first Tauri 2 desktop client/host for single-table Sit 'n Go No-Limit Texas Hold'em over real LAN TCP. The Rust backend owns networking, protocol compatibility, crypto, tournament orchestration, reconnect/resync, persistence, and state projection; the React + TypeScript frontend is a rendering shell.

## Current milestone status

The repository now contains:

- M0 workspace scaffold for Tauri 2 + Rust + React/TypeScript
- Rust module boundaries for `domain`, `engine`, `tournament`, `protocol`, `networking`, `crypto`, `storage`, `interop`, and `app_state`
- Tauri command/event bridge for frontend bootstrap state
- M1 domain/state foundation with immutable poker/tournament models, validators, participant capacity semantics, and state projection logic
- M2 protocol/crypto foundation with Android-shaped envelopes, canonical JSON signing bytes, replay protection, compact `pkr1_` join payload codec, and Ed25519/X25519/ChaCha utilities
- M3 real TCP runtime foundation with length-prefixed JSON framing, host/client join flow, encrypted private delivery, LAN IP validation, and transport tests
- M4 poker engine and tournament loop foundation with authoritative action windows, timeout handling, side-pot settlement, blind progression, and tournament lifecycle tests
- M5 reconnect/resync runtime with original-identity reconnect validation, reconnect-eligible disconnect handling, authoritative snapshot replacement, and sequence mismatch recovery tests
- M6 frontend shell flow with host/join/lobby/ready-room/table/history/complete/help/error surfaces, debug-gated internal tools, and launch-payload validation wiring
- M7 main-table UX polish with Rust-backed table projection rendering, observer mode, action tray confirmation flows, side-panel history/standings, and expanded frontend/runtime tests
- M8 multi-instance local testing support with isolated profile/session namespaces, debug-only launch helpers, and one-machine host/join/play coverage
- M9 Android interop audit against the current Android repo with documented findings and Android-aligned private-envelope key derivation
- M10 persistence, cached hand-history summaries, Tauri window-state restore, and Linux bundle output for release-readiness
- Architecture notes and frozen implementation choices aligned to the desktop specs

## Frozen implementation choices

- **Frontend stack:** React + TypeScript
- **Serialization path:** `serde` models plus one canonical JSON byte serializer path for signed bytes
- **Crypto stack:** `ed25519-dalek`, `x25519-dalek`, and `chacha20poly1305`
- **Transport framing:** length-prefixed JSON envelopes
- **Default LAN host port:** `43818` to match the current Android default
- **Per-instance storage:** profile namespace under a per-instance local data directory

## Tooling

### Required toolchains

- Rust stable toolchain
- Node.js 22+ and npm
- Tauri CLI (installed through the project `devDependencies`)

### Linux prerequisites for Tauri

Install the GTK/WebKit and build packages required by Tauri 2 on Linux. On Debian or Ubuntu, this is typically:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

If your distro uses different package names, install the WebKitGTK 4.1 and GTK 3 development packages that Tauri requires.

## Getting started

```bash
npm install
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run lint
npm run test
npm run tauri dev
```

## Build the app

For a production desktop build:

```bash
npm install
npm run tauri build
```

This compiles the frontend and Rust backend, then produces release bundles under `src-tauri/target/release/bundle/`.

If you only want the release binary without the platform bundles:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
```

## Run the app

For local development:

```bash
npm run tauri dev
```

For a compiled release binary:

```bash
./src-tauri/target/release/desktop-poker
```

You can also run a packaged Linux build directly, for example:

```bash
./src-tauri/target/release/bundle/appimage/Desktop\ Poker_0.1.0_amd64.AppImage
```

## Running multiple instances locally

The desktop app is intentionally designed for multiple concurrent instances. Use an explicit instance id so each launch gets its own storage namespace, session identity, and reconnect namespace:

```bash
DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev
DESKTOP_POKER_INSTANCE_ID=client-b npm run tauri dev
```

You can also pass the instance id on the app command line:

```bash
npm run tauri dev -- -- --instance-id client-c
```

Production binaries can also be launched multiple times with distinct ids:

```bash
./src-tauri/target/release/desktop-poker --instance-id host-a
./src-tauri/target/release/desktop-poker --instance-id client-b
```

The desktop shell also persists per-instance display name, host draft, recent direct-join payloads, saved hand-history summaries, and Tauri window state so repeated local sessions do not stomp one another.

## Passing a join payload at launch

M0 wires the launch contract into the bootstrap layer so later milestones can consume it from either environment or CLI input:

```bash
DESKTOP_POKER_JOIN_PAYLOAD='pkr1_...' npm run tauri dev
```

or

```bash
npm run tauri dev -- -- --join-payload 'pkr1_...'
```

The CLI form and env var are both surfaced through the Rust bootstrap state and made available to the frontend.

## Local multi-instance host/join flow

1. Launch a host instance with its own id, for example `DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev`.
2. Copy the current `pkr1_...` payload from the host/debug handoff surface.
3. Launch a second instance with a different id and either paste the payload on the Join screen or pass it on the command line:

```bash
DESKTOP_POKER_INSTANCE_ID=client-b npm run tauri dev -- -- --join-payload 'pkr1_...'
```

Loopback (`127.0.0.1`) flows are covered by the in-repo runtime tests, and local LAN flows use the same payload path once a connectable host IP is available. In debug builds, the Internal Tools screen can copy the payload directly and launch another instance with the payload already attached.

## Architecture notes

See [docs/DESKTOP_ARCHITECTURE.md](docs/DESKTOP_ARCHITECTURE.md) for the Rust/frontend ownership boundary, protocol compatibility stance, and multi-instance rules.

## Android interop status

See [docs/ANDROID_INTEROP_AUDIT.md](docs/ANDROID_INTEROP_AUDIT.md) for the current Android/Desktop audit.

Current status:

- protocol version and core envelope/join semantics were compared against the Android repo source
- Android-side crypto/networking/tournament tests were run during the audit
- mixed-runtime Android/Desktop host-client sessions are **not yet proven** in this repository

## Current limitations and release notes

- **Interop:** Android/Desktop interoperability is audited, but not fully proven until live mixed-runtime host/client sessions succeed.
- **Discovery:** room-code discovery is still absent/deferred for desktop MVP. Direct `pkr1_...` payload join is the supported path.
- **Network scope:** the app is LAN-only and assumes a trusted host-authoritative table on the local network.
- **Production behavior:** release builds default to the real TCP LAN runtime, and the debug launch helpers remain hidden outside debug builds.
- **Assets:** the table uses generated card/felt treatments and simple status badges so the MVP stays license-safe.
- **Sound:** MVP currently ships without sound. Any future sound support should remain optional and off by default.

## Linux bundles

`npm run tauri build` now produces:

- `src-tauri/target/release/bundle/deb/Desktop Poker_0.1.0_amd64.deb`
- `src-tauri/target/release/bundle/rpm/Desktop Poker-0.1.0-1.x86_64.rpm`
- `src-tauri/target/release/bundle/appimage/Desktop Poker_0.1.0_amd64.AppImage`
