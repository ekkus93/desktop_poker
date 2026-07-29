# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `672a4bcc76d3f6cd6d60eb5387047b1d6babb263`
- GitHub Actions run: `30414452548`
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
