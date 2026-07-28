# Latest Installed Debian Package Validation

- Overall result: **PASS**
- Validated commit: `50c0ad29052acccce18e1da7e84f8f1acbbab275`
- GitHub Actions run: `30409484117`
- Build outcome: `success`
- Validation outcome: `success`
- Evidence artifact: `linux-debian-package-v3-evidence`

- **PASS** — Debian package was produced
- **PASS** — Debian package installed through apt
- **PASS** — Package owns a valid executable, desktop entry, and icon files
- **PASS** — Installed executable passed the production WebDriver smoke
- **PASS** — Installed desktop entry launched the packaged process and X11 window
- **PASS** — Package purge removed the executable, desktop entry, and icon files

## Package

- Name: `desktop-poker`
- Version: `0.1.0`
- Architecture: `amd64`
- Debian SHA-256: `1b7528bd209d6d99ea1adfd0edca0d5664ead5468b21a408cc4cde6b71f21fab`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `3`
