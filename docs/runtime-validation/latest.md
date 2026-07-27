# Latest Linux Release Runtime Validation

- Result: **FAIL**
- Validated commit: `0b26a412341d42fdd44048ad05e2a9a90a7ca4bd`
- GitHub Actions run: `30232202361`
- Build outcome: `success`
- Runtime outcome: `failure`
- Recorded at: `2026-07-27T02:32:56.478641+00:00`
- Evidence artifact: `linux-release-runtime-evidence`

## Failure

AssertionError: Timed out waiting for route /host with text 'Host Tournament Setup'; last error: None

## Executed checks

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a WebDriver session
- **PASS** — Home rendered through the real Tauri backend
- **PASS** — browser mocks are unavailable in release mode
- **FAIL** — runtime smoke
