#!/usr/bin/env bash
# Build Cyberloom from this checkout and install it, with a menu entry.
#
#   scripts/install.sh              build, then install for this user (~/.local)
#   scripts/install.sh --system     the same, into /usr/local, with sudo
#   scripts/install.sh --uninstall  remove what a previous run put there
#
# Written for CachyOS and anything else that has pacman; on another
# distribution it skips the dependency step and tells you what it would have
# installed. Run it again after pulling to upgrade: the build is incremental
# and the install overwrites in place.
#
# What it puts where, for PREFIX = ~/.local or /usr/local:
#
#   PREFIX/bin/cyberloom                        the launcher (packaging/launcher.sh)
#   PREFIX/lib/cyberloom/cyberloom              the binary
#   PREFIX/lib/cyberloom/rigs/                  the four rigs, found by rigs_near()
#   PREFIX/share/applications/cyberloom.desktop the menu entry
#   PREFIX/share/icons/hicolor/512x512/apps/    the icon
#   PREFIX/share/cyberloom/graphs/              the example graphs
#
# The first run of the application serves ~/Cyberloom (see workspace_root() in
# apps/desktop/src-tauri/src/lib.rs). If that folder does not exist yet, this
# script creates it and copies the examples in, so the first window has
# something to look at. It never touches a ~/Cyberloom that already exists.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

prefix="$HOME/.local"
system=0
skip_deps=0
skip_build=0
uninstall=0

usage() {
  sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --system) system=1; prefix="/usr/local" ;;
    --prefix) shift; prefix="${1:?--prefix needs a directory}" ;;
    --skip-deps) skip_deps=1 ;;
    --no-build) skip_build=1 ;;
    --uninstall) uninstall=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# Everything that writes under the prefix goes through here, so a --system
# install asks for sudo exactly where it needs it and nowhere else.
as_owner() {
  if [ "$system" = 1 ] && [ "$(id -u)" != 0 ]; then sudo "$@"; else "$@"; fi
}

# ---------------------------------------------------------------- uninstall
if [ "$uninstall" = 1 ]; then
  say "Removing Cyberloom from $prefix"
  as_owner rm -f "$prefix/bin/cyberloom" \
    "$prefix/share/applications/cyberloom.desktop" \
    "$prefix/share/icons/hicolor/512x512/apps/cyberloom.png"
  as_owner rm -rf "$prefix/lib/cyberloom" "$prefix/share/cyberloom" "$prefix/share/licenses/cyberloom"
  command -v update-desktop-database >/dev/null && as_owner update-desktop-database -q "$prefix/share/applications" || true
  command -v gtk-update-icon-cache >/dev/null && as_owner gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" 2>/dev/null || true
  say "Done. ~/Cyberloom (your graphs) was left alone."
  exit 0
fi

# ------------------------------------------------------------- dependencies
# The same list as packaging/PKGBUILD: depends + makedepends. `rust` is added
# only when there is no cargo already, because the `rust` package conflicts
# with a rustup install, and pacman would refuse rather than pick one.
if [ "$skip_deps" = 0 ]; then
  packages=(base-devel git nodejs npm pkgconf webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg ffmpeg)
  command -v cargo >/dev/null || packages+=(rust)
  if command -v pacman >/dev/null; then
    # -Syu, not -S: on Arch a package list that has not been synced names
    # files the mirrors no longer have, and the install fails with a 404 on
    # something like enchant. Syncing without upgrading (-Sy) is worse — a
    # partial upgrade — so this is the full one. pacman still asks first.
    say "Syncing pacman and installing build and runtime dependencies (pacman will ask before changing anything)"
    sudo pacman -Syu --needed "${packages[@]}"
  else
    warn "no pacman here; make sure these are installed some other way: ${packages[*]}"
  fi
fi

# What the build needs, checked before it starts rather than twenty minutes in.
command -v cargo >/dev/null || die "cargo not found. Install the rust package, or rustup, and make sure ~/.cargo/bin is on your PATH."
command -v npm >/dev/null || die "npm not found. On CachyOS: sudo pacman -S nodejs npm"
node_major="$(node -p 'process.versions.node.split(".")[0]')"
[ "$node_major" -ge 22 ] || die "Node $(node --version) is too old; package.json wants >= 22."

# -------------------------------------------------------------------- build
cd "$here"
binary="$here/target/release/cyberloom"

if [ "$skip_build" = 0 ]; then
  # `npm ci` wipes node_modules and starts over, which is right the first
  # time and a waste every time after. Re-run it only when the lockfile has
  # changed since node_modules was last written.
  if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
    say "Installing npm packages"
    npm ci
  fi
  say "Building the shell"
  npm run build -w @cyberloom/desktop
  say "Building the host and the engine (the first time takes a while)"
  # `custom-protocol` is what makes this a release build in Tauri's eyes:
  # without it the window would look for the Vite dev server (see
  # apps/desktop/src-tauri/Cargo.toml).
  cargo build --release --bin cyberloom --features custom-protocol
fi

[ -x "$binary" ] || die "no binary at $binary; run without --no-build"

# ------------------------------------------------------------------ install
say "Installing to $prefix"
as_owner install -Dm755 "$binary" "$prefix/lib/cyberloom/cyberloom"
as_owner install -Dm755 packaging/launcher.sh "$prefix/bin/cyberloom"

# Replaced rather than merged, so a rig removed from the repository does not
# linger in an install that has been upgraded ten times.
as_owner rm -rf "$prefix/lib/cyberloom/rigs"
as_owner mkdir -p "$prefix/lib/cyberloom/rigs"
as_owner cp -r rigs/. "$prefix/lib/cyberloom/rigs/"

as_owner install -Dm644 apps/desktop/src-tauri/icons/icon.png \
  "$prefix/share/icons/hicolor/512x512/apps/cyberloom.png"
as_owner install -Dm644 LICENSE "$prefix/share/licenses/cyberloom/LICENSE"
as_owner mkdir -p "$prefix/share/cyberloom/graphs"
as_owner install -m644 fixtures/graphs/*.loom "$prefix/share/cyberloom/graphs/"

# The desktop file says `Exec=cyberloom` and trusts the PATH. A menu launcher
# does not necessarily have ~/.local/bin on its PATH, so the installed copy
# names the launcher by its full path.
as_owner mkdir -p "$prefix/share/applications"
sed "s|^Exec=cyberloom|Exec=$prefix/bin/cyberloom|" packaging/cyberloom.desktop \
  | as_owner tee "$prefix/share/applications/cyberloom.desktop" >/dev/null
as_owner chmod 644 "$prefix/share/applications/cyberloom.desktop"

# Best effort: a desktop that does not have these picks the entry up anyway,
# just later.
command -v update-desktop-database >/dev/null && as_owner update-desktop-database -q "$prefix/share/applications" || true
command -v gtk-update-icon-cache >/dev/null && as_owner gtk-update-icon-cache -q -t "$prefix/share/icons/hicolor" 2>/dev/null || true

# -------------------------------------------------------------- first run
if [ ! -e "$HOME/Cyberloom" ]; then
  mkdir -p "$HOME/Cyberloom/graphs"
  cp fixtures/graphs/*.loom "$HOME/Cyberloom/graphs/"
  say "Created ~/Cyberloom with the example graphs in it"
fi

# ------------------------------------------------------------------- report
say "Installed. Cyberloom is in your application menu, or run: cyberloom"
case ":$PATH:" in
  *":$prefix/bin:"*) ;;
  *)
    warn "$prefix/bin is not on your PATH, so the menu entry works but the command does not."
    case "${SHELL:-}" in
      *fish) echo "    fish_add_path $prefix/bin" ;;
      *) echo "    export PATH=\"$prefix/bin:\$PATH\"   # add to your shell's rc file" ;;
    esac
    ;;
esac
