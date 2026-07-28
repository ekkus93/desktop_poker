# Latest Installed Debian Package Validation

- Overall result: **PASS**
- Validated commit: `253b20a0b2555fa8a6b9cb3ccb33375bb8171e47`
- GitHub Actions run: `30381310082`
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
- Debian SHA-256: `088c3d49266c6f65a0e9d1cfdee0ef857059d9354b133f11a3f39a2328f3f220`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `3`
