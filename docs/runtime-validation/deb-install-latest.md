# Latest Installed Debian Package Validation

- Overall result: **PASS**
- Validated commit: `bda5515a07971c2a8832e96fba7a2c2a54c1efb1`
- GitHub Actions run: `38843947189`
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
- Debian SHA-256: `2164c95c73cd560c2e1fcbc37a07ac12329bd0b7623f52afc2bdfd512f0baa93`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `3`
