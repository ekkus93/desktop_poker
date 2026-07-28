# Latest Reconnect Protocol and Release Validation

- Result: **PASS**
- Validated commit: `6e730992a0f19b9f0d7f7486dd500e6e01d8b491`
- GitHub Actions run: `30349276304`
- Protocol tests: `success`
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
