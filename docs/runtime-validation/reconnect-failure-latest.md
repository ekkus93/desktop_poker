# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `2d1e2e4a7c3431b63f94b4b5c4e0042cd1918b81`
- GitHub Actions run: `30416201604`
- Protocol tests: `failure`
- Release build: `success`
- Release reconnect matrix: `failure`
- Evidence artifact: `reconnect-protocol-and-release-failure-evidence`

## Failure

AssertionError: Tauri command 'submit_table_action' failed: [object Object]

## Executed checks

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — unreachable-host join failed explicitly without retaining a client session
- **PASS** — lobby reconnect replaced the TCP tuple and restored a usable client session
- **FAIL** — release reconnect matrix
