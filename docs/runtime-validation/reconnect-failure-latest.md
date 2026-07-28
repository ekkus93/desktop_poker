# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `2432ceb80ec02916cf8ccb91a225f3f67aa6c95e`
- GitHub Actions run: `30332398233`
- Protocol tests: `success`
- Release build: `success`
- Release reconnect matrix: `failure`
- Evidence artifact: `reconnect-protocol-and-release-failure-evidence`

## Failure

AssertionError: Timed out waiting for Tauri command get_client_session_status; last error: None

## Executed checks

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — unreachable-host join failed explicitly without retaining a client session
- **PASS** — lobby reconnect replaced the TCP tuple and restored a usable client session
- **PASS** — active-hand reconnect restored the same hand on a new TCP tuple
- **PASS** — post-reconnect client action succeeded and immediate duplicate was rejected
- **FAIL** — release reconnect matrix
