# Latest Installed Debian Package Validation

- Overall result: **PASS**
- Validated commit: `abea9f5d60839dc96cf4787b36cf3f4f4566a52f`
- GitHub Actions run: `30394647314`
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
- Debian SHA-256: `bbf27f9fa2328b41b2b6c399c2ab018c63e9fbd3476a16ba292e9a267c483fd8`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `3`
