# Repository Guidelines

## Project Structure & Module Organization

The frontend lives in `src/`: `screens/` contains player flows, `components/` shared UI, `app/` shell and persistence, and `api/` the desktop bridge. Frontend unit and integration tests sit beside their modules as `*.test.ts(x)` files. `tests/browser/` contains Playwright browser checks. Rust is a workspace at the repository root: `crates/poker-core/` holds the shared poker engine, while `src-tauri/` contains the desktop backend and its Rust integration tests. Shared fixtures are in `src/fixtures/`; product and validation documentation is in `docs/`.

## Build, Test, and Development Commands

Use Node.js 24 and stable Rust.

- `npm run dev` starts the Vite frontend; `npm run tauri dev` runs the desktop app.
- `npm run build` type-checks TypeScript and builds the frontend; `npm run tauri build` packages the desktop app.
- `npm run test` runs Vitest; `npm run test:geometry` runs Playwright browser geometry checks.
- `npm run lint` runs ESLint; `npm run format:check` checks Prettier formatting.
- `cargo test --workspace --all-targets --all-features` runs Rust tests; `cargo fmt --check` checks Rust formatting; `cargo clippy --workspace --all-targets --all-features -- -D warnings` runs Rust linting.

## Coding Style & Naming Conventions

Use Prettier (two spaces) and ESLint with zero warnings. Name components and screens in PascalCase, hooks with a `use` prefix, and tests `*.test.ts(x)`. Format Rust with `rustfmt`; use `snake_case` for modules, functions, and variables, and `PascalCase` for types. Gameplay authority, networking, protocol, crypto, and persistence belong in Rust; keep the frontend focused on presentation and interaction.

## Testing Guidelines

Add or update tests beside changed modules; use `tests/browser/` for browser-level layout behavior. Run relevant tests while iterating and the full suite before submitting. Ignored LLM integration tests require local Ollama (`cargo test --workspace -- --ignored`). No fixed coverage threshold is configured; cover affected behavior.

## Commit & Pull Request Guidelines

History uses prefixes such as `fix(networking): ...` and `docs: ...`. Write specific, imperative subjects. Pull requests should explain the behavior change and its user impact, link related issues or validation records, and include screenshots for visible UI changes. Record relevant test and build results, including any checks not run.

## Security & Configuration

Never commit API keys or local provider secrets. Provider keys belong in the app's protected secret storage; use a dedicated low-privilege development key. Keep protocol and Android interoperability changes aligned with the compatibility guidance in `CLAUDE.md` and `docs/`.
