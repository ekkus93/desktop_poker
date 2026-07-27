# Embedded NPC Tournament CI Contract

The `Embedded tiny-model NPC validation` workflow must run a complete release-mode tournament with the in-process GGUF provider.

The tournament gate uses the same production Tauri commands, host runtime, NPC runner, prompt/profile loading, llama.cpp backend, legal-action validation, and authoritative action submission paths as the desktop application.

Acceptance requirements:

- the pinned GGUF model passes its SHA-256 check;
- the release binary builds and launches through `tauri-driver`;
- two profile-backed embedded NPCs are seated and ready;
- the tournament settles at least two hands and reaches completion;
- both NPC identities produce committed actions;
- at least three embedded NPC actions are observed in the authoritative public event stream;
- final standings contain the human host and both NPCs;
- no `[npc-runner]` fallback, inference, stale-window, submission, or panic diagnostic is emitted;
- machine-readable evidence is published to `docs/runtime-validation/embedded-tournament-latest.json`.

The test model is downloaded only in GitHub Actions and is not bundled with release artifacts.
