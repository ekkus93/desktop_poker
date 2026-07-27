# Latest Linux Release Runtime Validation

- Overall result: **FAIL**
- Validated commit: `373aa53c1fa1290b8ded58f14ef2e424c43ea95e`
- GitHub Actions run: `30233994857`
- Build outcome: `success`
- Single-instance outcome: `success`
- Multi-instance outcome: `failure`
- Recorded at: `2026-07-27T03:17:15.897914+00:00`
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

Failure: WebDriverError: WebDriver POST /session/b16e0fb4-f2ef-49c6-994d-31a723a3a7e3/element returned HTTP 404: {"value":{"error":"no such element","message":"","stacktrace":""}}

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — conflict host release instance launched
- **PASS** — three instance IDs and profile directories are distinct
- **PASS** — client host draft is independently namespaced before joining
- **PASS** — host started a real TCP session and produced a pkr1_ invite
- **PASS** — client joined the live host over real TCP
- **FAIL** — multi-instance runtime smoke
