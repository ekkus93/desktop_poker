# Latest Linux Release Keychain Validation

- Result: **PASS**
- Validated commit: `2584caa591d9eb5e584e020e0626d0becd04d5a2`
- GitHub Actions run: `38888221672`
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

