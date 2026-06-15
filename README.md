# Desktop Poker

[![CI](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml/badge.svg)](https://github.com/ekkus93/desktop_poker/actions/workflows/ci.yml)

Desktop Poker is a Linux-first Tauri 2 desktop client/host for single-table Sit 'n Go No-Limit Texas Hold'em over real LAN TCP. The Rust backend owns networking, protocol compatibility, crypto, tournament orchestration, reconnect/resync, persistence, and state projection; the React + TypeScript frontend is a rendering shell.

## Current product status

The repository currently includes:

- A desktop host and join flow for a single-table Sit 'n Go over LAN TCP.
- A player-first frontend built around `Home`, `Host`, `Join`, `Lobby`, `Main Table`, `History`, `Tournament Complete`, `Settings`, and contextual recovery states.
- A Rust backend that owns poker rules, tournament orchestration, networking, reconnect and resync behavior, protocol compatibility, crypto, persistence, and state projection.
- Launch-payload handling for direct join flows using compact `pkr1_...` invitations.
- **NPC players** with two decision engines:
  - A rule-based engine (preflop hand tiers, postflop hand category analysis) used when no LLM is configured or as a fallback on API errors.
  - An LLM-driven engine that calls an external language model, builds a full per-hand prompt from a loaded AI profile, and validates the response against the legal action set before submission.
- **Multi-provider LLM support:** Anthropic (Claude), OpenAI, Ollama (local), and llama-server (local). Ollama and llama-server need no API key and run entirely offline.
- **AI profiles** — per-NPC Markdown persona files with YAML frontmatter. Three built-in starters are included (Aggressive Alice, Conservative Carlos, Balanced Sam). Profiles can be created, edited, and deleted through the Settings → AI Profiles screen. Profiles can include optional `## Opponent tendencies` and `## Tilt behaviour` sections.
- **Session memory and opponent modelling:** after each hand the NPC runner accumulates session history (wins/losses, pots, bluffs), opponent stats (VPIP, preflop raise frequency, aggression factor, showdown win rate), and tilt state (consecutive loss streaks). This context is injected into the LLM prompt on each subsequent decision.
- Debug-gated internal tools and multi-instance local testing support without exposing those flows as normal player UX.
- Cached local shell state, saved hand-history summaries, and per-instance storage and window-state isolation.
- Linux desktop bundle output through Tauri 2.
- Frontend and Rust test coverage for host, join, table, observer, reconnect, persistence, transport, NPC decision logic, LLM client, prompt assembly, profile parsing, session history, opponent stats, and tilt state.

This repository should be treated as an MVP in progress rather than a production-ready poker release. Automated coverage is strong, but manual multi-instance desktop QA and live Android/Desktop interoperability QA are still tracked separately below.

## Current player flow

1. Home: choose `Host Tournament` or `Join Tournament`
2. Host or Join setup: open a table or bring an invite and confirm the destination
3. Lobby: gather the table, check readiness, and start the first hand
4. Main Table: play the hand while the board, pot, and current decision stay front and center
5. History, Help, Recovery, or Complete: review secondary details only when you choose them or when the game needs them

Recovery and reconnect states appear only when needed, and debug or multi-instance tooling stays on a hidden debug-only route outside the normal player path in debug/dev usage.

## NPC players

The host can add AI-controlled players when setting up a tournament. Each NPC seat uses either the rule-based engine or an LLM-based engine depending on whether a provider is configured and an AI profile is assigned.

### LLM provider setup

Open **Settings → LLM provider** to choose a provider and enter the required credentials. The config is written to `{app_data_dir}/llm-provider.json` and loaded at startup.

| Provider | API key required | Default endpoint | Default model |
|---|---|---|---|
| Anthropic (Claude) | Yes | `https://api.anthropic.com` | `claude-haiku-4-5-20251001` |
| OpenAI | Yes | `https://api.openai.com` | `gpt-4o-mini` |
| Ollama | No | `http://localhost:11434` | `llama3.2` |
| llama-server | No | `http://localhost:8080` | *(loaded model)* |

All four providers use the same prompt format. Anthropic uses the `/v1/messages` API; the other three use the OpenAI-compatible `/v1/chat/completions` endpoint.

You can override the endpoint URL and model name per-provider in the settings UI, or write the config file directly:

```json
{
  "provider": "ollama",
  "apiKey": null,
  "endpointUrl": null,
  "model": "llama3.2"
}
```

The config file location is `~/.local/share/desktop-poker/llm-provider.json` on Linux. The legacy `claude-api-key.txt` file from earlier builds is auto-migrated to an Anthropic provider config on first startup.

### AI profiles

Profiles are Markdown files with YAML frontmatter stored under `{app_data_dir}/npc-profiles/`. Three starter profiles are bundled:

- `aggressive-alice` — loose-aggressive, bets large, bluffs rivers
- `conservative-carlos` — tight-passive, only plays premiums, never bluffs
- `balanced-sam` — GTO-approximating, adjusts sizing to board texture

Manage profiles through **Settings → AI Profiles**. The editor supports the full profile format including optional named sections:

```markdown
---
name: My Player
style: loose-aggressive
skill: intermediate
---
General strategy description.

## Opponent tendencies
How to adjust based on observed opponent stats (VPIP, AF, etc.).

## Tilt behaviour
How this player behaves after a losing streak.
```

### Session memory

Once an LLM provider is configured, the runner builds context automatically across hands:

- **Session history** — win/loss streak, chip trajectory, recent key hands (bluffs caught, big pots).
- **Opponent stats** — VPIP, preflop raise frequency, aggression factor, showdown win rate per opponent (shown after ≥3 hands observed).
- **Tilt state** — `None` → `Mild` (2 consecutive losses) → `Full` (3+ consecutive losses).

This context is injected between the profile body and the game state in each LLM prompt. If the assembled prompt exceeds ~6,000 tokens it is progressively trimmed (opponent context dropped first, then session history truncated, tilt state always preserved).

### LLM fallback

If the provider returns an error, a timeout, unparseable JSON, or an illegal action, the NPC falls back to the rule-based engine for that decision. No hand is blocked by a failed LLM call.

## What the app is responsible for

### Frontend

- Presents the player flow and screen hierarchy.
- Renders host, join, lobby, table, history, completion, help, recovery, settings, and AI-profile management surfaces.
- Persists lightweight local shell state such as recent payloads, host draft values, and cached hand-history summaries.

### Rust backend

- Validates tournament configuration and join payloads.
- Runs the authoritative poker engine and tournament lifecycle.
- Owns real LAN TCP transport, reconnect and resync logic, protocol framing, signing and encryption helpers, and per-instance bootstrap metadata.
- Projects player-safe table views for the frontend.
- Runs the NPC decision loop in a background thread: observes the authoritative state, submits actions via the host server, tracks session history and opponent stats, writes tilt state to a shared slot readable by the debug inspector.

### Internal tooling

- Supports debug review, payload handoff, and multi-instance development workflows.
- Remains intentionally separated from the normal player path and lives on a hidden debug-only route.

## Frozen implementation choices

- **Frontend stack:** React + TypeScript
- **Serialization path:** `serde` models plus one canonical JSON byte serializer path for signed bytes
- **Crypto stack:** `ed25519-dalek`, `x25519-dalek`, and `chacha20poly1305`
- **Transport framing:** length-prefixed JSON envelopes
- **Default LAN host port:** `43818` to match the current Android default
- **Per-instance storage:** profile namespace under a per-instance local data directory
- **LLM timeout:** 20 seconds per request (accommodates local CPU-only inference); failures fall back to rule-based engine

## Tooling

### Required toolchains

- Rust stable toolchain
- Node.js 24 and npm
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

The desktop app is intentionally designed for multiple concurrent instances. Use an explicit instance id so each launch gets its own storage namespace, session identity, and reconnect namespace.

For production or compiled binaries, you can launch as many instances as you want directly:

```bash
./src-tauri/target/release/desktop-poker --instance-id host-a
./src-tauri/target/release/desktop-poker --instance-id client-b
```

For local development, do not run `npm run tauri dev` twice. The first `tauri dev` process starts the shared Vite dev server on port `1420`, and a second `tauri dev` will fail when it tries to start that same port again.

Use this workflow instead:

```bash
DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev
```

Keep that first terminal running. Then launch each additional development instance against the already-running dev server by starting the Rust app directly:

```bash
DESKTOP_POKER_INSTANCE_ID=client-b cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --
DESKTOP_POKER_INSTANCE_ID=client-c cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --
```

You can also pass the instance id on the app command line for those extra development launches:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features -- --instance-id client-c
```

The desktop shell also persists per-instance display name, host draft, recent direct-join payloads, saved hand-history summaries, and Tauri window state so repeated local sessions do not stomp one another.

## Passing a join payload at launch

The app accepts a launch payload from either environment or CLI input so the join flow can open with a shared `pkr1_...` invitation already attached:

```bash
DESKTOP_POKER_JOIN_PAYLOAD='pkr1_...' npm run tauri dev
```

or

```bash
npm run tauri dev -- -- --join-payload 'pkr1_...'
```

The CLI form and env var are both surfaced through the Rust bootstrap state and consumed by the frontend join and recovery flows.

## Local multi-instance host/join flow

1. Launch the shared frontend dev server and the host instance with its own id, for example `DESKTOP_POKER_INSTANCE_ID=host-a npm run tauri dev`.
2. Copy the current `pkr1_...` payload from the host flow or from the hidden debug route in debug builds.
3. Launch a second development instance with a different id by running the Rust app directly against the already-running dev server, then either paste the payload on the Join screen or pass it on the command line:

```bash
DESKTOP_POKER_INSTANCE_ID=client-b cargo run --manifest-path src-tauri/Cargo.toml --no-default-features -- --join-payload 'pkr1_...'
```

Loopback (`127.0.0.1`) flows are covered by the in-repo runtime tests, and local LAN flows use the same payload path once a connectable host IP is available. In debug builds, the hidden debug route can copy the payload directly and launch another instance with the payload already attached.

## Architecture notes

See [docs/DESKTOP_ARCHITECTURE.md](docs/DESKTOP_ARCHITECTURE.md) for the Rust/frontend ownership boundary, protocol compatibility stance, and multi-instance rules.

## Android interop status

See [docs/ANDROID_INTEROP_AUDIT.md](docs/ANDROID_INTEROP_AUDIT.md) for the current Android/Desktop audit.

Current status:

- Core protocol version, join-payload shape, canonical JSON rules, reconnect metadata, and key-derivation behavior were compared against the Android source.
- Android-side crypto, networking, and tournament tests were run during the audit.
- Mixed-runtime Android/Desktop host-client sessions are still **audited but not yet proven** by live end-to-end runs in this repository.

## Manual QA status

- **Desktop multi-instance QA:** not yet recorded as complete in this repository for the current repair pass.
- **Android/Desktop interoperability QA:** not yet recorded as complete in this repository.
- **Debug/probe reachability QA in release builds:** still required as an explicit manual check even though release gating is covered by automated tests.
- **LLM NPC live session QA:** API key configuration, profile CRUD in UI, and LLM NPC in a live session are tracked as manual QA items. Rule-based fallback has been exercised in automated tests; live LLM decisions require a manual session with a configured provider.

## Release notes and current limitations

What is supported today:

- Single-table Sit 'n Go No-Limit Texas Hold'em hosted from the desktop app over LAN TCP.
- Direct join via a shared `pkr1_...` payload.
- Reconnect and resync behavior within the desktop runtime.
- Multi-instance local testing with isolated per-instance storage and session identity.
- NPC players with rule-based decisions (always available) or LLM-driven decisions (when a provider is configured and a profile is assigned).
- AI profile management: create, edit, and delete persona profiles with optional opponent-tendency and tilt-behaviour sections.
- Session memory: NPC decisions incorporate running hand history, opponent stats, and tilt state across the tournament.
- LLM provider support: Anthropic Claude, OpenAI, Ollama (local), and llama-server (local). Any OpenAI-compatible endpoint can be used by setting a custom endpoint URL.
- Linux development and bundle output through Tauri 2.

What is intentionally limited right now:

- **Interop:** Android/Desktop interoperability is audited, but not fully proven until live mixed-runtime sessions succeed.
- **Discovery:** there is no room-code discovery or matchmaking flow yet. Direct `pkr1_...` payload join is the supported path.
- **Network scope:** the app is LAN-only and assumes a trusted host-authoritative table on the local network.
- **Production behavior:** release builds use the real TCP LAN runtime, while debug and probe helpers are internal-only and should not be treated as player-facing features.
- **Assets:** the table uses generated card and felt treatments plus simple status badges so the MVP stays license-safe.
- **Sound:** sound is currently not included. Any future sound support should remain optional and off by default.
- **LLM response quality:** the NPC decision quality depends on the model and prompt. Smaller local models may not reliably output valid JSON; the rule-based fallback handles any failed or illegal response.

## Linux bundles

`npm run tauri build` produces Linux release bundles under `src-tauri/target/release/bundle/`.

Current bundle targets:

- `src-tauri/target/release/bundle/deb/Desktop Poker_0.1.0_amd64.deb`
- `src-tauri/target/release/bundle/rpm/Desktop Poker-0.1.0-1.x86_64.rpm`
- `src-tauri/target/release/bundle/appimage/Desktop Poker_0.1.0_amd64.AppImage`

These bundle names reflect the current package version and will change when the app version changes.
