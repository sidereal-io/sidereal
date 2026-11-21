#!/bin/sh
# Apply the weekly mise tool update to the working tree. The publish job of
# mise-tool-updates.yml runs it on the patch from the update job.
#
#   scripts/apply-mise-tool-update.sh PATCH
set -eu
[ $# -eq 1 ] || { echo "usage: apply-mise-tool-update.sh PATCH" >&2; exit 2; }
patch=$1
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

# The update job leaves an empty patch when nothing changed.
[ -s "$patch" ] || exit 0
git apply "$patch"
