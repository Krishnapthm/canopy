#!/bin/sh
# Modified for Canopy.
# Installs Canopy, and herdr-core if there's none yet:
#
#   curl -fsSL https://github.com/Krishnapthm/canopy/releases/latest/download/install.sh | sh
#
# Linux: the app into ~/.local/share/canopy, `canopy` in ~/.local/bin, a
# launcher entry and icon. macOS: Canopy.app into ~/Applications, keeping a
# Developer ID signature intact (ad hoc for unsigned builds). The core, with
# its own installer, into ~/.local/bin only if none is there yet; a core you
# already have is left alone. Running this again updates the app.
# CANOPY_RELEASES_URL serves the app from elsewhere (tests, mirrors).
set -eu

releases=${CANOPY_RELEASES_URL:-https://github.com/Krishnapthm/canopy/releases/latest/download}
bin="$HOME/.local/bin"

say() { printf '%s\n' "$*"; }
fail() { say "canopy: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || fail "needs $1"; }
need curl
need tar

case "$(uname -s)" in
  Linux) os=linux ;;
  Darwin) os=macos ;;
  MINGW* | MSYS* | CYGWIN*) fail "there is no Windows build yet; build from source: https://github.com/Krishnapthm/canopy#build-from-source" ;;
  *) fail "no build for $(uname -s)" ;;
esac
case "$(uname -m)" in
  x86_64 | amd64) arch=x86_64 ;;
  arm64 | aarch64) arch=aarch64 ;;
  *) fail "no build for $(uname -m)" ;;
esac
if [ "$os" = macos ] && [ "$arch" = x86_64 ]; then
  # Rosetta reports x86_64 even on Apple Silicon.
  if [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ] ||
     [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then
    arch=aarch64
  else
    fail "Intel Macs aren't supported; build from source: https://github.com/Krishnapthm/canopy#build-from-source"
  fi
fi
target="$os-$arch"

sha256() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# SHA256SUMS lists every archive of the release; `sha256sum -c` format.
curl -fsSL "$releases/SHA256SUMS" -o "$tmp/SHA256SUMS" || fail "cannot reach $releases"
archive="canopy-$target.tar.gz"
want=$(sed -n "s/^\([0-9a-f]\{64\}\)  $archive\$/\1/p" "$tmp/SHA256SUMS")
[ -n "$want" ] || fail "no build for $target yet"

say "Installing Canopy for $target..."
curl -fsSL "$releases/$archive" -o "$tmp/app.tar.gz" || fail "download failed"
[ "$(sha256 "$tmp/app.tar.gz")" = "$want" ] || fail "download is damaged (checksum mismatch); try again"
tar -xzf "$tmp/app.tar.gz" -C "$tmp"
mkdir -p "$bin"

if [ "$os" = linux ]; then
  data=${XDG_DATA_HOME:-$HOME/.local/share}
  home="$data/canopy"
  mkdir -p "$data"
  rm -rf "$home.new"
  mv "$tmp/canopy" "$home.new"
  rm -rf "$home"
  mv "$home.new" "$home"
  ln -sfn "$home/canopy" "$bin/canopy"
  mkdir -p "$data/applications" "$data/icons/hicolor/scalable/apps"
  cp "$home/canopy.svg" "$data/icons/hicolor/scalable/apps/canopy.svg"
  cat > "$data/applications/canopy.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Canopy
Comment=Terminals and coding agents that keep running
Exec=$bin/canopy
Icon=canopy
Terminal=false
Categories=Development;
StartupWMClass=canopy
EOF
  command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$data/applications" 2>/dev/null || true
  [ -f "$data/icons/hicolor/icon-theme.cache" ] && gtk-update-icon-cache -f -t "$data/icons/hicolor" >/dev/null 2>&1 || true
  # Plasma remembers a missing icon until it is told the theme changed,
  # so a fresh install shows an empty page in the taskbar.
  command -v dbus-send >/dev/null 2>&1 &&
    dbus-send --session --type=signal /KIconLoader org.kde.KIconLoader.iconChanged int32:0 2>/dev/null || true
else
  apps="$HOME/Applications"
  app="$apps/Canopy.app"
  mkdir -p "$apps"
  # Prepared in the temp folder, so Spotlight never indexes a half-made
  # bundle under a temporary name in Applications.
  # Never change a certificate-signed bundle: macOS permissions follow its
  # signing identity. Only unsigned/ad-hoc development builds need this PATH.
  signed=false
  if signature=$(/usr/bin/codesign -dvv "$tmp/Canopy.app" 2>&1); then
    if printf '%s\n' "$signature" | grep -q '^Authority=Developer ID Application:'; then
      signed=true
    elif ! printf '%s\n' "$signature" | grep -qx 'Signature=adhoc'; then
      fail "cannot determine app signature: $signature"
    fi
  elif ! printf '%s\n' "$signature" | grep -q ': code object is not signed at all$'; then
    fail "cannot determine app signature: $signature"
  fi
  if [ "$signed" = true ]; then
    /usr/bin/codesign --verify --deep --strict "$tmp/Canopy.app" || fail "invalid app signature"
  else
    /usr/libexec/PlistBuddy -c "Add :LSEnvironment:PATH string $bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin" "$tmp/Canopy.app/Contents/Info.plist"
    /usr/bin/codesign --force --sign - "$tmp/Canopy.app" >/dev/null 2>&1
  fi
  rm -rf "$app"
  mv "$tmp/Canopy.app" "$app"
  ln -sfn "$app/Contents/MacOS/Canopy" "$bin/canopy"
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$app" 2>/dev/null || true
fi

# The core: installed only if there's none yet. One that's already there
# (yours, herdr's, or a package manager's) is left alone. Its own installer,
# saved first: a failed download piped into sh would look like success.
if [ ! -e "$bin/herdr-core" ] && ! command -v herdr-core >/dev/null 2>&1; then
  say "Installing herdr-core, the terminal engine Canopy runs on..."
  if curl -fsSL https://releases.herdr.dev/core/install.sh -o "$tmp/core.sh"; then
    HC_INSTALL_DIR="$bin" sh "$tmp/core.sh" || say "herdr-core didn't install; Canopy will show how when it opens."
  else
    say "Cannot reach releases.herdr.dev; Canopy will show how to install herdr-core when it opens."
  fi
fi

say "Canopy is installed. Open it from your apps, or run: canopy"
case ":$PATH:" in
  *":$bin:"*) ;;
  *) say "Add $bin to your PATH to use canopy and hc from a terminal." ;;
esac
