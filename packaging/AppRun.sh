#!/bin/sh
# The AppImage's entry point.
#
# One thing to do beyond starting the binary, and it is the reason this file
# exists: WebKitGTK's DMABUF renderer is broken on several drivers — most
# visibly on NVIDIA under Wayland, where the window comes up black — and the
# fix is an environment variable that has to be set before the process starts.
#
# It is set only when nothing has set it, so anyone who knows better still wins,
# and only when the session is Wayland, because on X11 the accelerated path is
# the one that works and turning it off would cost frames for nothing.
if [ -z "$WEBKIT_DISABLE_DMABUF_RENDERER" ] && [ "$XDG_SESSION_TYPE" = "wayland" ]; then
  WEBKIT_DISABLE_DMABUF_RENDERER=1
  export WEBKIT_DISABLE_DMABUF_RENDERER
fi

# GTK picks its backend on its own and gets it right; this is here so that
# `GDK_BACKEND=x11 cyberloom` works as an escape hatch on a session where
# Wayland is misbehaving, which is the first thing to try before filing a bug.
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/cyberloom" "$@"
