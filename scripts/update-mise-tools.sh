#!/bin/sh
# Update the mise tool pins, as the weekly mise-tool-updates workflow does.
# Changes the working tree only; it commits, pushes and opens nothing.
# Needs the network. Run it through mise: `mise exec -- scripts/update-mise-tools.sh`.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

# rustup owns the toolchain release, and the lock records only the channel.
mise upgrade --exclude rust
# Let's Spec has an exact pin, so only --bump moves it and rewrites mise.toml.
mise upgrade --bump github:sidereal-io/lets-spec
# mise upgrade may record only this machine's platform for a new release.
mise lock
mise exec -- lets sync
