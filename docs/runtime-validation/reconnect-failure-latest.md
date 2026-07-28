# Latest Reconnect Protocol and Release Validation

- Result: **PASS**
- Validated commit: `5a52bfe9813cf74d562a5d72fdbe85713d0a0cda`
- GitHub Actions run: `30358343712`
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
