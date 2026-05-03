# Desktop Poker

Desktop Poker is a Linux-first Tauri 2 desktop client/host for single-table Sit 'n Go No-Limit Texas Hold'em over real LAN TCP. The Rust backend owns networking, protocol compatibility, crypto, tournament orchestration, reconnect/resync, persistence, and state projection; the React + TypeScript frontend is a rendering shell.

## Current milestone status

The repository now contains:

- M0 workspace scaffold for Tauri 2 + Rust + React/TypeScript
- Rust module boundaries for `domain`, `engine`, `tournament`, `protocol`, `networking`, `crypto`, `storage`, `interop`, and `app_state`
- Tauri command/event bridge for frontend bootstrap state
- M1 domain/state foundation with immutable poker/tournament models, validators, participant capacity semantics, and state projection logic
- M2 protocol/crypto foundation with Android-shaped envelopes, canonical JSON signing bytes, replay protection, compact `pkr1_` join payload codec, and Ed25519/X25519/ChaCha utilities
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

## Running multiple instances locally

The desktop app is intentionally designed for multiple concurrent instances. Use an explicit instance id so each launch gets its own storage namespace:

```bash
DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev
DESKTOP_POKER_INSTANCE_ID=client-b npm run tauri dev
```

You can also pass the instance id on the app command line:

```bash
npm run tauri dev -- -- --instance-id client-c
```

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

## Architecture notes

See [docs/DESKTOP_ARCHITECTURE.md](docs/DESKTOP_ARCHITECTURE.md) for the Rust/frontend ownership boundary, protocol compatibility stance, and multi-instance rules.
