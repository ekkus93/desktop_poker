# Latest Installed Debian Package Validation

- Overall result: **FAIL**
- Validated commit: `7bd7257b3806bb57e468b23f977cededcb997fcf`
- GitHub Actions run: `30325479550`
- Build outcome: `success`
- Validation outcome: `failure`
- Evidence artifact: `linux-debian-package-v2-evidence`

- **PASS** — Debian package was produced
- **PASS** — Debian package installed through apt
- **PASS** — Package owns a valid executable, desktop entry, and icons
- **PASS** — Installed executable passed the production WebDriver smoke
- **PASS** — Installed desktop entry launched the packaged process and X11 window
- **FAIL** — validation command failed with status 1 near line 1

## Package

- Name: `desktop-poker`
- Version: `0.1.0`
- Architecture: `amd64`
- Debian SHA-256: `0c7599c58eb8eee63a9ff1cfba0b298cb41bb1dff48c937c81e9ba710f90aa73`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `10`
