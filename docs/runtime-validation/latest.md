# Latest Linux Release Runtime Validation

- Overall result: **FAIL**
- Validated commit: `253b20a0b2555fa8a6b9cb3ccb33375bb8171e47`
- GitHub Actions run: `30381309998`
- Build outcome: `success`
- Single-instance outcome: `success`
- Multi-instance outcome: `failure`
- Recorded at: `2026-07-28T17:09:02.996972+00:00`
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

Failure: AssertionError: Timed out waiting for source text 'Invite looks good'; last error: None

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — conflict host release instance launched
- **PASS** — three instance IDs and profile directories are distinct
- **PASS** — client host draft is independently namespaced before joining
- **PASS** — host started a real TCP session and produced a pkr1_ invite
- **FAIL** — multi-instance runtime smoke
