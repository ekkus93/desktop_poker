# Latest Linux Release Runtime Validation

- Result: **PASS**
- Validated commit: `76de807ffb8d365d6b576098ae4af74f892a07a7`
- GitHub Actions run: `30232598478`
- Build outcome: `success`
- Runtime outcome: `success`
- Recorded at: `2026-07-27T02:41:42.833519+00:00`
- Evidence artifact: `linux-release-runtime-evidence`

## Executed checks

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
