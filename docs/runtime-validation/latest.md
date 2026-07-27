# Latest Linux Release Runtime Validation

- Overall result: **FAIL**
- Validated commit: `164bae0700d0e8926a836412d62d492f9046eaa8`
- GitHub Actions run: `30233131900`
- Build outcome: `success`
- Single-instance outcome: `success`
- Multi-instance outcome: `failure`
- Recorded at: `2026-07-27T02:56:44.475647+00:00`
- Evidence artifact: `linux-release-runtime-evidence`

## Single-instance release smoke

Result: **PASS**

- **PASS** — tauri-driver became ready
- **PASS** — release binary created a WebDriver session
- **PASS** — Home rendered through the real Tauri backend
- **PASS** — browser mocks are unavailable in release mode
- **PASS** — Host route rendered
- **PASS** — Join route rendered
- **PASS** — invalid invite failed explicitly: The invite could not be checked.
- **PASS** — /settings route rendered
- **PASS** — /rules route rendered
- **PASS** — session guard redirected /lobby without a live session
- **PASS** — session guard redirected /table without a live session
- **PASS** — release /debug route is not reachable
- **PASS** — runtime screenshot and final page source captured

## Live multi-instance release smoke

Result: **FAIL**

Failure: WebDriverError: tauri-driver did not become ready: timed out

- **FAIL** — multi-instance runtime smoke
