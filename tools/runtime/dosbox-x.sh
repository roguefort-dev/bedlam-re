#!/bin/sh
# Pinned DOSBox-X for the P4 differential harness (see docs/RUNTIME.md).
# Flathub user install contained inside the repo via XDG_DATA_HOME - gitignored,
# no system installs, no auto-updates. Pin: commit fa89039ca01aca36d9031f287d69b885d7510fb24499e9c33e1db420ab6ccdb2 (v2026.08.02).
set -e
REPO_ROOT=$(cd "$(dirname "$0")/../.." && pwd)
export XDG_DATA_HOME="$REPO_ROOT/runtime/xdg"
exec flatpak run com.dosbox_x.DOSBox-X "$@"
