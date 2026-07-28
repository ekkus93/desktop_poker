# Latest Reconnect Protocol and Release Validation

- Result: **FAIL**
- Validated commit: `46c30c22d7ad42cd68dbd9b95c70836e2155d70a`
- GitHub Actions run: `30379734661`
- Protocol tests: `failure`
- Release build: `success`
- Release reconnect matrix: `success`
- Evidence artifact: `reconnect-protocol-and-release-failure-evidence`

## Executed checks

- **PASS** — host release instance launched
- **PASS** — client release instance launched
- **PASS** — unreachable-host join failed explicitly without retaining a client session
- **PASS** — lobby reconnect replaced the TCP tuple and restored a usable client session
- **PASS** — active-hand reconnect restored the same hand on a new TCP tuple
- **PASS** — post-reconnect client action succeeded and immediate duplicate was rejected
- **PASS** — active-hand host loss became terminal and rejected stale table/action access
- **PASS** — lobby host loss became terminal and rejected stale table/action access
