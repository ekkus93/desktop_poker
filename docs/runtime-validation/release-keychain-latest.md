# Latest Linux Release Keychain Validation

- Result: **FAIL**
- Validated commit: `88c1b86e02adea8e450b55bb24d894467a1b4237`
- GitHub Actions run: `30330069272`
- Release build: `success`
- Secret Service persistence path: `success`
- Unavailable-keychain path: `failure`
- Evidence artifact: `linux-release-keychain-evidence`

## Persistence and clear

- **PASS** — fresh release process started without a configured provider
- **PASS** — release credential was accepted while public config remained non-secret
- **PASS** — global app-data files and runtime log contain no credential or plaintext key file
- **PASS** — release restart recovered the credential through the OS keychain
- **PASS** — clear removed provider settings and the keychain credential
- **PASS** — second release restart confirmed durable keychain deletion

## Failure behavior

WebDriverError: WebDriver POST /session/64f839bb-0340-4a31-af2f-5e4d58b3d325/execute/async error could not write provider key for anthropic: keychain write failed: Platform secure storage failure: DBus error: Failed to connect to socket /home/runner/work/_temp/desktop-poker-missing-secret-service-bus: No such file or directory: None
- **FAIL** — release keychain failure path

