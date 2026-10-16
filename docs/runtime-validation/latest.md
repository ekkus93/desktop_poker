# Latest Linux Release Runtime Validation

- Overall result: **FAIL**
- Validated commit: `1cf8f1a5a94afc22111d52ef50749b98ca74d595`
- GitHub Actions run: `38884571826`
- Build outcome: `success`
- Single-instance outcome: `failure`
- Multi-instance outcome: `success`
- Recorded at: `2026-10-16T10:05:16.367821+00:00`
- Evidence artifact: `linux-release-runtime-evidence`

## Single-instance release smoke

Result: **FAIL**

Failure: AssertionError: Page source missing expected text: ['Check invite', 'Continue to lobby']

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a WebDriver session
- **PASS** — Home rendered through the real Tauri backend
- **PASS** — browser mocks are unavailable in release mode
- **PASS** — Host route rendered
- **FAIL** — runtime smoke

## Live multi-instance release smoke

Result: **PASS**

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — conflict host release instance launched
- **PASS** — three instance IDs and profile directories are distinct
- **PASS** — client host draft is independently namespaced before joining
- **PASS** — host started a real TCP session and produced a pkr1_ invite
- **PASS** — client joined the live host over real TCP with matching seat indexes
- **PASS** — host began the lobby in its authoritative occupied seat
- **PASS** — client claimed the remaining open seat through the release UI
- **PASS** — host and client agree on distinct authoritative seat assignments
- **PASS** — ready state propagated to both release instances
- **PASS** — both instances entered Main Table after authoritative start
- **PASS** — private cards are isolated and public table state is synchronized
- **PASS** — same-port host failed explicitly: Unable to start hosting on TCP port 43818. Unable to start hosting.
- **PASS** — conflicting host recovered successfully on port 43819
