# Latest Linux Release Keychain Validation

- Result: **PASS**
- Validated commit: `94c8afbdef7f2a3ee484479f01990f38c97d41cc`
- GitHub Actions run: `30392417948`
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

could not write provider key for anthropic: keychain write failed: Platform secure storage failure: DBus error: Failed to connect to socket /home/runner/work/_temp/desktop-poker-missing-secret-service-bus: No such file or directory
- **PASS** — unavailable keychain produced an explicit release error
- **PASS** — failed keychain write created no provider state or plaintext fallback

