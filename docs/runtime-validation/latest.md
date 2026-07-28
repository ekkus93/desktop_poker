# Latest Linux Release Runtime Validation

- Overall result: **FAIL**
- Validated commit: `5fe5db0d30e72a8e34c5c5b0de1a61117f5e886e`
- GitHub Actions run: `30378367204`
- Build outcome: `success`
- Single-instance outcome: `success`
- Multi-instance outcome: `failure`
- Recorded at: `2026-07-28T16:31:29.723451+00:00`
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
