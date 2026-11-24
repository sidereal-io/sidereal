#!/bin/sh
# Deletes this checkout's Rust build output, server/target/, when it is
# larger than a size limit, and keeps it otherwise.
#
#   sh server/scripts/target-clean.sh
#   SIDEREAL_TARGET_LIMIT_MIB=2048 sh server/scripts/target-clean.sh
set -eu

say() { printf 'target-clean: %s\n' "$*"; }
die() {
  printf 'target-clean: %s\n' "$*" >&2
  exit 1
}

root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
target=$root/server/target
limit=$SIDEREAL_TARGET_LIMIT_MIB

if [ ! -e "$target" ]; then
  say "there is no $target: nothing to delete"
  exit 0
fi

# du -sk reports the disk space allocated, in KiB; round up to MiB.
kib=$(du -sk "$target")
kib=${kib%%[!0-9]*}
mib=$(((kib + 1023) / 1024))

if [ "$mib" -gt "$limit" ]; then
  rm -rf -- "$target"
  say "$target is $mib MiB, over the limit of $limit MiB: deleted it"
else
  say "$target is $mib MiB, at or under the limit of $limit MiB: kept it"
fi
