# Latest Linux Release Runtime Validation

- Overall result: **PASS**
- Validated commit: `56c787030de7303879006e132ae28087a0ebcbfb`
- GitHub Actions run: `30315356891`
- Build outcome: `success`
- Single-instance outcome: `success`
- Multi-instance outcome: `success`
- Recorded at: `2026-07-27T23:51:37.236785+00:00`
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
- **PASS** — same-port host failed explicitly: Unable to start hosting.
- **PASS** — conflicting host recovered successfully on port 43819
