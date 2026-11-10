#!/usr/bin/env bash
# Installs the Ubuntu packages a Linux machine needs before it builds or
# tests Solis. The one list for every workflow: each Linux job that builds
# a crate runs this first, so a job can't be left without a package the
# others have (the weekly whole-game check was, and never built).
#
# Usage: linux-packages.sh           what any build needs
#        linux-packages.sh window    that, and what the window test and the
#                                    GPU tests need to run
#
# For Ubuntu and Debian (it uses apt-get). CONTRIBUTING.md, Setting up,
# names the same packages for people: change both together.
set -euo pipefail

# The sound crate reaches the sound card through ALSA on Linux and builds
# against its headers. Every crate that uses sound needs them to build
# (the editor and the app among them), even when no sound is played.
build=(libasound2-dev)

# A virtual screen, a software GPU driver, tools to click and take
# screenshots, and the keyboard library a window needs on X11.
window=(xvfb xdotool imagemagick mesa-vulkan-drivers libxkbcommon-x11-0)

case "${1:-}" in
  "") packages=("${build[@]}") ;;
  window) packages=("${build[@]}" "${window[@]}") ;;
  *)
    echo "linux-packages.sh: unknown set '$1'. Usage: linux-packages.sh [window]" >&2
    exit 2
    ;;
esac

# CI's machines install as an ordinary user with sudo; a container often
# runs as root and has no sudo at all.
as_root() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  else
    sudo "$@"
  fi
}

as_root apt-get update
as_root apt-get install -y "${packages[@]}"
