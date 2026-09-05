#!/bin/sh
# What `cyberloom` on the PATH runs when the application was installed by
# `scripts/install.sh`. The real binary lives in `../lib/cyberloom/`, beside
# the rigs, which is one of the layouts `rigs_near()` in
# `crates/loomd/src/run/runner.rs` knows how to find.
#
# This file exists for the same one reason `AppRun.sh` does. WebKitGTK's DMABUF
# renderer is broken on several drivers — most visibly NVIDIA under Wayland,
# where the window comes up black with nothing in any log — and the fix is an
# environment variable that has to be set before the process starts. An
# AppImage gets that from `AppRun.sh`; a binary installed on the PATH and
# launched from a desktop menu gets it from here, or from nowhere.
#
# Set only when nothing has set it, so anyone who knows better still wins, and
# only on Wayland, because on X11 the accelerated path is the one that works.
if [ -z "$WEBKIT_DISABLE_DMABUF_RENDERER" ] && [ "$XDG_SESSION_TYPE" = "wayland" ]; then
  WEBKIT_DISABLE_DMABUF_RENDERER=1
  export WEBKIT_DISABLE_DMABUF_RENDERER
fi

# `GDK_BACKEND=x11 cyberloom` remains the escape hatch on a Wayland session
# that is misbehaving; GTK reads it on its own, nothing to do here.
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/../lib/cyberloom/cyberloom" "$@"
