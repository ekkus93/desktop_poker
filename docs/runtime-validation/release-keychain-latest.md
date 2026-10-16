# Latest Linux Release Keychain Validation

- Result: **PASS**
- Validated commit: `88489f26252c9a32e41e9611695d8b7cdb4a46d1`
- GitHub Actions run: `38890639554`
- Release build: `success`
- Secret Service persistence path: `success`
- Unavailable-keychain path: `success`
- Evidence artifact: `linux-release-keychain-evidence`

## Persistence and clear

- **PASS** — fresh release process started without a configured provider
- **PASS** — release credential was accepted while public config remained non-secret
- **PASS** — global app-data files and runtime log contain no credential or plaintext key file
- **PASS** — release restart recovered the credential through the OS keychain
- **PASS** — clear removed provider settings and the keychain credential
- **PASS** — second release restart confirmed durable keychain deletion

## Failure behavior

- **PASS** — unavailable keychain produced an explicit release error
- **PASS** — failed keychain write created no provider state or plaintext fallback

