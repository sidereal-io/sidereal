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

# The limit is checked before anything is measured, so a bad value never
# deletes. Set but empty counts as a bad value.
limit=${SIDEREAL_TARGET_LIMIT_MIB-4096}
case $limit in
  '' | *[!0-9]*)
    die "SIDEREAL_TARGET_LIMIT_MIB must be a whole number of MiB, such as 4096; got '$limit'. Deleting nothing."
    ;;
esac
# Drop leading zeros, so 010 reads as ten. 0 stays 0.
limit=${limit#"${limit%%[!0]*}"}
limit=${limit:-0}
# Larger numbers overflow the shell's arithmetic; no disk is that large.
if [ "${#limit}" -gt 15 ]; then
  die "SIDEREAL_TARGET_LIMIT_MIB is too large; use at most 15 digits. Deleting nothing."
fi

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
