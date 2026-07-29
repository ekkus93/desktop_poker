# Latest Installed Debian Package Validation

- Overall result: **PASS**
- Validated commit: `9b4bb0a5738e609c418b6dfd673dd431dd7cb70e`
- GitHub Actions run: `30432686451`
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
- Debian SHA-256: `e80919ffd0a3c78e6376010f53eabf3561de436d71be54a056133f2c6450e6de`
- Installed binary: `/usr/bin/desktop-poker`
- Desktop file: `/usr/share/applications/Desktop Poker.desktop`
- Installed icon count: `3`
