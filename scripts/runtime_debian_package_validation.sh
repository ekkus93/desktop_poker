#!/usr/bin/env bash
# Build-output validator for an installed Desktop Poker Debian package.
#
# The caller must build the .deb first and install the desktop/WebDriver test
# dependencies. This script installs the package, validates both direct and
# desktop-entry launch paths, purges it, and records durable evidence.
set -Eeuo pipefail

evidence_dir=${DEBIAN_PACKAGE_EVIDENCE_DIR:-"$PWD/debian-package-evidence"}
driver_port=${DEBIAN_PACKAGE_DRIVER_PORT:-4604}
native_port=${DEBIAN_PACKAGE_NATIVE_PORT:-4605}
mkdir -p "$evidence_dir"
: > "$evidence_dir/steps.tsv"
: > "$evidence_dir/metadata.env"

command -v gtk-launch | tee "$evidence_dir/gtk-launch-path.txt"
command -v xdotool | tee "$evidence_dir/xdotool-path.txt"
command -v tauri-driver | tee "$evidence_dir/tauri-driver-path.txt"

record() {
  printf 'PASS\t%s\n' "$1" | tee -a "$evidence_dir/steps.tsv"
}

package_name=""
cleanup() {
  status=$?
  pkill -f '^/usr/bin/desktop-poker' 2>/dev/null || true
  if [[ -n "$package_name" ]] && dpkg-query -W "$package_name" >/dev/null 2>&1; then
    sudo apt-get purge -y "$package_name" \
      >"$evidence_dir/emergency-purge.log" 2>&1 || true
  fi
  if [[ $status -ne 0 ]]; then
    printf 'FAIL\tvalidation command failed with status %s near line %s\n' \
      "$status" "${BASH_LINENO[0]:-unknown}" >> "$evidence_dir/steps.tsv"
  fi
  exit "$status"
}
trap cleanup EXIT

deb=$(find target/release/bundle/deb -maxdepth 1 -type f -name '*.deb' -print -quit)
test -n "$deb"
deb=$(realpath "$deb")
package_name=$(dpkg-deb -f "$deb" Package)
package_version=$(dpkg-deb -f "$deb" Version)
package_arch=$(dpkg-deb -f "$deb" Architecture)
deb_sha256=$(sha256sum "$deb" | awk '{print $1}')
record "Debian package was produced"

dpkg-deb --info "$deb" | tee "$evidence_dir/deb-info.txt"
dpkg-deb --contents "$deb" | tee "$evidence_dir/deb-contents.txt"
file "$deb" | tee "$evidence_dir/deb-file.txt"
sha256sum "$deb" | tee "$evidence_dir/deb-sha256.txt"

sudo apt-get install -y "$deb" 2>&1 | tee "$evidence_dir/deb-install.log"
dpkg-query -W -f='${Status}\n' "$package_name" | grep -Fx 'install ok installed'
record "Debian package installed through apt"

dpkg-query -L "$package_name" | sort | tee "$evidence_dir/installed-files.txt"
installed_binary=$(dpkg-query -L "$package_name" | awk '/^\/usr\/bin\// { print; exit }')
desktop_file=$(dpkg-query -L "$package_name" | awk '/^\/usr\/share\/applications\/.*\.desktop$/ { print; exit }')
mapfile -t icon_files < <(
  dpkg-query -L "$package_name" \
    | awk '/^\/usr\/share\/(icons|pixmaps)\/.*\.(png|svg|xpm)$/ { print }'
)
test -x "$installed_binary"
test -f "$desktop_file"
test "${#icon_files[@]}" -gt 0
desktop-file-validate "$desktop_file" 2>&1 | tee "$evidence_dir/desktop-file-validate.log"
grep -E '^Exec=' "$desktop_file" | tee "$evidence_dir/desktop-exec.txt"
grep -E '^Icon=' "$desktop_file" | tee "$evidence_dir/desktop-icon.txt"
printf '%s\n' "${icon_files[@]}" | tee "$evidence_dir/package-icon-files.txt"
record "Package owns a valid executable, desktop entry, and icon files"

installed_sha256=$(sha256sum "$installed_binary" | awk '{print $1}')
desktop_id=$(basename "$desktop_file" .desktop)
{
  printf 'deb=%s\n' "$deb"
  printf 'packageName=%s\n' "$package_name"
  printf 'packageVersion=%s\n' "$package_version"
  printf 'packageArchitecture=%s\n' "$package_arch"
  printf 'debSha256=%s\n' "$deb_sha256"
  printf 'installedBinary=%s\n' "$installed_binary"
  printf 'installedBinarySha256=%s\n' "$installed_sha256"
  printf 'desktopFile=%s\n' "$desktop_file"
  printf 'desktopId=%s\n' "$desktop_id"
  printf 'iconCount=%s\n' "${#icon_files[@]}"
} >> "$evidence_dir/metadata.env"

export INSTALLED_BINARY="$installed_binary"
export DESKTOP_ID="$desktop_id"
export DESKTOP_POKER_BINARY_SHA256="$installed_sha256"
export DESKTOP_POKER_INSTANCE_ID="installed-deb-webdriver-${GITHUB_RUN_ID:-local}"
export DEBIAN_PACKAGE_EVIDENCE_DIR="$evidence_dir"
export DEBIAN_PACKAGE_DRIVER_PORT="$driver_port"
export DEBIAN_PACKAGE_NATIVE_PORT="$native_port"

xvfb-run -a --server-args="-screen 0 1440x960x24" \
  dbus-run-session -- bash -s <<'RUNTIME'
set -Eeuo pipefail
export NO_AT_BRIDGE=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
export GDK_BACKEND=x11

tauri-driver \
  --port "$DEBIAN_PACKAGE_DRIVER_PORT" \
  --native-port "$DEBIAN_PACKAGE_NATIVE_PORT" \
  >"$DEBIAN_PACKAGE_EVIDENCE_DIR/tauri-driver.log" 2>&1 &
driver_pid=$!
cleanup_runtime() {
  pkill -f "^${INSTALLED_BINARY}" 2>/dev/null || true
  kill "$driver_pid" 2>/dev/null || true
  wait "$driver_pid" 2>/dev/null || true
}
trap cleanup_runtime EXIT

python3 scripts/runtime_webdriver_smoke.py \
  --application "$INSTALLED_BINARY" \
  --driver-url "http://127.0.0.1:${DEBIAN_PACKAGE_DRIVER_PORT}" \
  --evidence-dir "$DEBIAN_PACKAGE_EVIDENCE_DIR"
mv \
  "$DEBIAN_PACKAGE_EVIDENCE_DIR/release-runtime-result.json" \
  "$DEBIAN_PACKAGE_EVIDENCE_DIR/installed-runtime-result.json"

pkill -f "^${INSTALLED_BINARY}" 2>/dev/null || true
for _ in $(seq 1 40); do
  pgrep -f "^${INSTALLED_BINARY}" >/dev/null 2>&1 || break
  sleep 0.25
done
! pgrep -f "^${INSTALLED_BINARY}" >/dev/null 2>&1

update-desktop-database /usr/share/applications
DESKTOP_POKER_INSTANCE_ID="installed-deb-desktop-${GITHUB_RUN_ID:-local}" \
  gtk-launch "$DESKTOP_ID" \
  >"$DEBIAN_PACKAGE_EVIDENCE_DIR/desktop-entry-launch.log" 2>&1 &
launcher_pid=$!

process_found=false
window_found=false
: > "$DEBIAN_PACKAGE_EVIDENCE_DIR/window-ids.txt"
for _ in $(seq 1 80); do
  if pgrep -f "^${INSTALLED_BINARY}" >/dev/null 2>&1; then
    process_found=true
  fi
  if xdotool search --name 'Desktop Poker' \
    >"$DEBIAN_PACKAGE_EVIDENCE_DIR/window-ids.txt" 2>&1; then
    window_found=true
  fi
  if [[ "$process_found" = true && "$window_found" = true ]]; then
    break
  fi
  if ! kill -0 "$launcher_pid" 2>/dev/null \
    && ! pgrep -f "^${INSTALLED_BINARY}" >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

test "$process_found" = true
test "$window_found" = true
pgrep -af "^${INSTALLED_BINARY}" \
  | tee "$DEBIAN_PACKAGE_EVIDENCE_DIR/desktop-entry-processes.txt"
while read -r window_id; do
  xdotool getwindowname "$window_id"
done < "$DEBIAN_PACKAGE_EVIDENCE_DIR/window-ids.txt" \
  | tee "$DEBIAN_PACKAGE_EVIDENCE_DIR/window-titles.txt"
pkill -f "^${INSTALLED_BINARY}" 2>/dev/null || true
RUNTIME
record "Installed executable passed the production WebDriver smoke"
record "Installed desktop entry launched the packaged process and X11 window"

sudo apt-get purge -y "$package_name" 2>&1 | tee "$evidence_dir/deb-purge.log"
! dpkg-query -W "$package_name" >/dev/null 2>&1
test ! -e "$installed_binary"
test ! -e "$desktop_file"
for icon_file in "${icon_files[@]}"; do
  test ! -e "$icon_file"
done
record "Package purge removed the executable, desktop entry, and icon files"
package_name=""
