# Latest Linux Release Keychain Validation

- Result: **FAIL**
- Validated commit: `34508dfb62b239a012489004397548580dd24698`
- GitHub Actions run: `30328090726`
- Release build: `success`
- Secret Service persistence path: `failure`
- Unavailable-keychain path: `failure`
- Evidence artifact: `linux-release-keychain-evidence`

## Persistence and clear

AssertionError: profile directory does not exist: /home/runner/.local/share/desktop-poker/profiles/release-keychain-success-30328090726
- **PASS** — fresh release profile started without a configured provider
- **FAIL** — release keychain persistence

## Failure behavior

AssertionError: 'set_llm_api_key' unexpectedly succeeded
- **FAIL** — release keychain failure path

