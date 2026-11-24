#!/bin/sh
# Deletes this checkout's Rust build output, server/target/, when it is
# larger than a size limit, and keeps it otherwise. It deletes nothing
# when server or server/target is a link, when server, server/target or
# a folder inside it is a mount point, or when it cannot measure the
# folder.
# CONTRIBUTING.md explains the limit.
#
#   sh server/scripts/target-clean.sh
#   SIDEREAL_TARGET_LIMIT_MIB=2048 sh server/scripts/target-clean.sh
set -eu

say() { printf 'target-clean: %s\n' "$*"; }
die() {
  printf 'target-clean: %s\n' "$*" >&2
  exit 1
}

# $(...) drops every trailing newline, which could turn a checkout path
# ending in one into a sibling checkout's path. The x keeps them; then
# only pwd's own newline is removed.
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P && echo x) ||
  die "cannot find the repo root. Deleting nothing."
root=${root%x}
root=${root%?}
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

# Deleting a link would free nothing, and following it could delete a
# folder that other checkouts share. A link at server would make
# server/target another checkout's folder.
if [ -L "$root/server" ]; then
  die "$root/server is a symbolic link, not this checkout's own folder. Deleting nothing."
fi
if [ -L "$target" ]; then
  die "$target is a symbolic link, not this checkout's own folder. Deleting nothing."
fi

if [ ! -e "$target" ]; then
  say "there is no $target: nothing to delete"
  exit 0
fi

# rm -rf crosses mount points, and a bind mount from the same file system
# has its parent's device number, so read the system mount table. Refuse
# a mount at server/target, inside it, or at server: a bind mount there
# would make server/target another checkout's folder.
server_phys=$(CDPATH= cd -P -- "$root/server" && pwd -P) ||
  die "cannot find the physical path of $root/server. Deleting nothing."
target_phys=$(CDPATH= cd -P -- "$target" && pwd -P) ||
  die "cannot find the physical path of $target. Deleting nothing."

# mount prints these characters escaped, such as \040 for a space, so the
# paths below cannot be matched reliably. Refuse rather than guess.
tab=$(printf '\t')
nl=$(printf '\nx')
nl=${nl%x}
case $target_phys in
  *' '* | *"$tab"* | *"$nl"* | *\\*)
    die "the path $target_phys holds a space, tab, newline or backslash, so its mount points cannot be checked. Deleting nothing. Move the checkout to a path without those characters."
    ;;
esac

mounts=$(mount) || die "cannot read the mount table: 'mount' failed. Deleting nothing."
[ -n "$mounts" ] || die "cannot read the mount table: 'mount' printed nothing. Deleting nothing."

set +e
printf '%s\n' "$mounts" |
  grep -qF -e " on $target_phys " -e " on $target_phys/" -e " on $server_phys "
found=$?
set -e
case $found in
  0) die "$server_phys, $target_phys or a folder inside it is a mount point, and rm -rf would cross into it. Deleting nothing." ;;
  1) ;;
  *) die "cannot search the mount table. Deleting nothing." ;;
esac

# du -sk reports the disk space allocated, in KiB; round up to MiB.
kib=$(du -sk "$target") || die "cannot measure $target: du failed. Deleting nothing."
kib=${kib%%[!0-9]*}
[ -n "$kib" ] || die "cannot measure $target: du printed no size. Deleting nothing."
mib=$(((kib + 1023) / 1024))

if [ "$mib" -gt "$limit" ]; then
  rm -rf -- "$target" ||
    die "deleting $target failed; the folder may be partly deleted. Its next build rebuilds what is missing."
  say "$target is $mib MiB, over the limit of $limit MiB: deleted it"
else
  say "$target is $mib MiB, at or under the limit of $limit MiB: kept it"
fi
