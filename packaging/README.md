# Packaging

Two targets, both Linux, in the order the plan puts them: an AppImage that runs
anywhere, then an AUR package for the machine this was built for.

## AppImage

```sh
npm ci
npm run build            # the shell
cargo tauri build        # the host, the engine, and the bundle
```

The bundle carries `rigs/` beside the binary, which is where the engine looks
for them first (`shipped_rigs()` in `crates/loomd/src/run/runner.rs`). A rig the
user adds lives in their workspace and takes precedence over one with the same
name here.

`AppRun.sh` sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Wayland sessions where
nothing else has set it. That is not superstition: WebKitGTK's DMABUF path is
broken on several drivers and the symptom is a window that comes up black, with
nothing in any log to say why. It is left alone on X11, where the accelerated
path is the one that works.

## The install script

`scripts/install.sh` is for the machine in front of you: it runs `pacman` for
the dependencies, builds the shell and the host from the checkout, and installs
the result under `~/.local` (or `/usr/local` with `--system`), with a menu
entry. Running it again is an upgrade; `--uninstall` reverses it. It is what to
use until there is a release for the `PKGBUILD` to fetch, and it is quicker than
the `PKGBUILD` afterwards too, because it builds in the checkout and so builds
incrementally.

It installs the binary and the rigs together in `lib/cyberloom/`, which is the
first place `rigs_near()` looks, and `launcher.sh` as `bin/cyberloom`. The
launcher does for an installed binary what `AppRun.sh` does for the AppImage:
sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Wayland when nothing else has, so a
menu entry on an NVIDIA machine opens a window rather than a black rectangle.

If `~/Cyberloom` does not exist yet, the script creates it with the four example
graphs in it, so the first window has something on the canvas. A `~/Cyberloom`
that already exists is never touched.

## AUR

`PKGBUILD` builds from a release tarball. `ffmpeg` is a hard dependency — a
sense that cannot run is worse than a larger install — and Python and Ollama are
optional, because a graph with neither a Python block nor a model block never
asks for them and the settings screen says so plainly when one does.

## First run

There is no installer step and no project file. The host serves, in order:

1. `CYBERLOOM_WORKSPACE`, if it is set;
2. the current directory, *if it contains graphs or a `workspace.yaml`* — which
   is what running from a checkout means;
3. `~/Cyberloom`, created on the spot.

The second condition is the one that matters here. An AppImage started from a
desktop menu has whatever working directory the launcher felt like, often `/` or
the home folder, and serving that would mean either an empty window or an offer
to write graphs into somebody's home directory.

What is missing is not an error. The settings screen detects Python, ffmpeg,
Ollama and the model weights every time it opens, says what each one is for, and
gives the command that fixes it. Offline is a supported state: every graph that
does not need what is missing runs, and the ones that do say so on the block.
