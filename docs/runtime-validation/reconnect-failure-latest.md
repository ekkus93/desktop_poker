# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `e8538efd8dbc6b82cc06dc1a164f1f09235e9528`
- GitHub Actions run: `30408526027`
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
