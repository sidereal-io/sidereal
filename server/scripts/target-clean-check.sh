#!/bin/sh
# Checks `sh server/scripts/target-clean.sh` without a Rust toolchain or
# Docker. Each case copies the script into its own scratch repo layout,
# under mktemp -d, gives it a server/target/ folder holding a 2 MiB file
# of random data, runs it, and checks its exit status, its output and
# what is left on disk.
#
#   sh server/scripts/target-clean-check.sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)

fx=$(mktemp -d "${TMPDIR:-/tmp}/target-clean-check.XXXXXX")
trap 'rm -rf "$fx"' EXIT
trap 'exit 130' HUP INT TERM

failures=0
fail() {
  echo "FAIL [$label]: $*" >&2
  failures=$((failures + 1))
}

# new_case LABEL: starts a case with a fresh repo layout at $repo.
new_case() {
  label=$1
  repo=$fx/cases/$label
  mkdir -p "$repo/server/scripts"
  cp "$root/server/scripts/target-clean.sh" "$repo/server/scripts/"
}

# fill DIR: writes a 2 MiB file of random data to DIR/blob. Random data
# keeps a sparse file or a compressing file system from shrinking it.
fill() {
  mkdir -p "$1"
  dd if=/dev/urandom of="$1/blob" bs=1024 count=2048 2>/dev/null
}

# run_clean [NAME=VALUE...]: runs the clean-up script from $repo with
# only the given variables added to a clean environment, and records
# its exit status in $status and its output in $fx/out and $fx/err.
run_clean() {
  set +e
  (
    unset SIDEREAL_TARGET_LIMIT_MIB CARGO_TARGET_DIR
    cd "$repo" && env "$@" sh server/scripts/target-clean.sh
  ) >"$fx/out" 2>"$fx/err"
  status=$?
  set -e
}

expect_status_zero() {
  [ "$status" -eq 0 ] || fail "exit status $status, expected 0; stderr: $(cat "$fx/err")"
}
expect_status_nonzero() {
  [ "$status" -ne 0 ] || fail "exit status 0, expected non-zero; stdout: $(cat "$fx/out")"
}
# expect_out TEXT / expect_err TEXT: the output holds TEXT.
expect_out() {
  grep -qF -- "$1" "$fx/out" || fail "stdout lacks '$1': $(cat "$fx/out")"
}
expect_err() {
  grep -qF -- "$1" "$fx/err" || fail "stderr lacks '$1': $(cat "$fx/err")"
}
expect_kept() {
  [ -f "$1/blob" ] || fail "$1/blob was deleted"
}
expect_gone() {
  [ ! -e "$1" ] || fail "$1 still exists"
}

# stub NAME BODY: writes a command NAME, running BODY, into $bin, which
# a case puts first on the clean-up script's PATH.
stub() {
  bin=$fx/bin/$label
  mkdir -p "$bin"
  printf '#!/bin/sh\n%s\n' "$2" >"$bin/$1"
  chmod +x "$bin/$1"
}

# stub_mount LINE: a mount that prints a root mount, then LINE.
stub_mount() {
  mkdir -p "$fx/bin/$label"
  printf '%s\n' "/dev/sda1 on / type ext4 (rw,relatime)" "$1" >"$fx/bin/$label/table"
  stub mount "cat \"$fx/bin/$label/table\""
}

# phys DIR: DIR's physical path, as mount prints it.
phys() {
  (CDPATH= cd -P -- "$1" && pwd -P)
}

# The folder's size as the script measures it: du -sk, rounded up to MiB.
size_mib() {
  kib=$(du -sk "$1")
  kib=${kib%%[!0-9]*}
  echo $(((kib + 1023) / 1024))
}

# --- Size against the limit ---------------------------------------------

new_case over-limit
fill "$repo/server/target"
mib=$(size_mib "$repo/server/target")
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_zero
expect_gone "$repo/server/target"
expect_out "$mib MiB"
expect_out "limit of 1 MiB"
expect_out "deleted"

new_case under-limit
fill "$repo/server/target"
mib=$(size_mib "$repo/server/target")
run_clean SIDEREAL_TARGET_LIMIT_MIB=10
expect_status_zero
expect_kept "$repo/server/target"
expect_out "$mib MiB"
expect_out "limit of 10 MiB"
expect_out "kept"

new_case at-limit
fill "$repo/server/target"
mib=$(size_mib "$repo/server/target")
run_clean SIDEREAL_TARGET_LIMIT_MIB="$mib"
expect_status_zero
expect_kept "$repo/server/target"
expect_out "kept"

new_case no-folder
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_zero
expect_out "nothing to delete"

# --- The limit ----------------------------------------------------------

new_case default-limit
fill "$repo/server/target"
run_clean
expect_status_zero
expect_kept "$repo/server/target"
expect_out "limit of 4096 MiB"

n=0
for bad in 2G -1 1.5 ' 1' '' 1234567890123456; do
  n=$((n + 1))
  new_case "invalid-limit-$n"
  fill "$repo/server/target"
  run_clean SIDEREAL_TARGET_LIMIT_MIB="$bad"
  expect_status_nonzero
  expect_err "SIDEREAL_TARGET_LIMIT_MIB"
  expect_kept "$repo/server/target"
done

# Leading zeros are a whole number, read as decimal, not octal.
new_case leading-zeros
fill "$repo/server/target"
run_clean SIDEREAL_TARGET_LIMIT_MIB=010
expect_status_zero
expect_kept "$repo/server/target"
expect_out "limit of 10 MiB"

# --- Only this checkout's own folder ------------------------------------

new_case cargo-target-dir-elsewhere
fill "$fx/elsewhere"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 CARGO_TARGET_DIR="$fx/elsewhere"
expect_status_zero
expect_kept "$fx/elsewhere"

# A cargo first on PATH records any call: cargo clean would follow
# CARGO_TARGET_DIR and could delete a shared folder.
new_case no-cargo-clean
fill "$repo/server/target"
stub cargo "touch \"$fx/cargo-called\"; exit 1"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_zero
expect_gone "$repo/server/target"
[ ! -e "$fx/cargo-called" ] || fail "the script called cargo"

new_case link
fill "$fx/link-dest"
ln -s "$fx/link-dest" "$repo/server/target"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_nonzero
expect_err "symbolic link"
[ -L "$repo/server/target" ] || fail "the link was deleted"
expect_kept "$fx/link-dest"

# A link at server would make server/target another checkout's folder.
new_case server-link
mkdir -p "$fx/server-link-dest/server/scripts"
cp "$root/server/scripts/target-clean.sh" "$fx/server-link-dest/server/scripts/"
fill "$fx/server-link-dest/server/target"
rm -rf "$repo/server"
ln -s "$fx/server-link-dest/server" "$repo/server"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_nonzero
expect_err "symbolic link"
[ -L "$repo/server" ] || fail "the link was deleted"
expect_kept "$fx/server-link-dest/server/target"

# The script finds the repo from its own path, not the current folder.
new_case run-from-elsewhere
fill "$repo/server/target"
mkdir -p "$fx/elsewhere-cwd/server/target"
set +e
(cd "$fx/elsewhere-cwd" && SIDEREAL_TARGET_LIMIT_MIB=1 sh "$repo/server/scripts/target-clean.sh") >"$fx/out" 2>"$fx/err"
status=$?
set -e
expect_status_zero
expect_gone "$repo/server/target"
[ -d "$fx/elsewhere-cwd/server/target" ] || fail "the current folder's server/target was deleted"

# --- Mount points -------------------------------------------------------
# A real mount needs root, so a mount first on PATH prints a mount table
# with a root line and the case's line, in the form Linux prints.

new_case mount-target
fill "$repo/server/target"
stub_mount "/dev/sdb1 on $(phys "$repo/server/target") type ext4 (rw,relatime)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "mount point"
expect_kept "$repo/server/target"

new_case mount-inside-target
fill "$repo/server/target"
mkdir -p "$repo/server/target/debug/cache"
stub_mount "/dev/sdb1 on $(phys "$repo/server/target")/debug/cache type none (rw,bind)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "mount point"
expect_kept "$repo/server/target"

new_case mount-server
fill "$repo/server/target"
stub_mount "/dev/sdb1 on $(phys "$repo/server") type ext4 (rw,relatime)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "mount point"
expect_kept "$repo/server/target"

# macOS and the BSDs print "<source> on <mount point> (<fs>, <options>)".
new_case mount-target-bsd
fill "$repo/server/target"
stub_mount "/dev/disk3s1 on $(phys "$repo/server/target") (apfs, local)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "mount point"
expect_kept "$repo/server/target"

new_case mount-fails
fill "$repo/server/target"
stub mount "echo 'mount: stub failure' >&2; exit 1"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "'mount' failed"
expect_kept "$repo/server/target"

new_case mount-prints-nothing
fill "$repo/server/target"
stub mount "exit 0"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "'mount' printed nothing"
expect_kept "$repo/server/target"

# A mount at the repo root makes the whole checkout one tree, so the
# folder is still this checkout's own.
new_case mount-repo-root
fill "$repo/server/target"
stub_mount "/dev/sdb1 on $(phys "$repo") type ext4 (rw,relatime)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_zero
expect_gone "$repo/server/target"

new_case mount-sibling
fill "$repo/server/target"
mkdir -p "$repo/server/target-other"
stub_mount "/dev/sdb1 on $(phys "$repo/server/target-other") type ext4 (rw,relatime)"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_zero
expect_gone "$repo/server/target"

# mount prints a space as \040, so the script cannot match such a path.
new_case "path with space"
fill "$repo/server/target"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_nonzero
expect_err "space, tab, newline or backslash"
expect_kept "$repo/server/target"

new_case 'path\with-backslash'
fill "$repo/server/target"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1
expect_status_nonzero
expect_err "space, tab, newline or backslash"
expect_kept "$repo/server/target"

# --- Failures while measuring or deleting -------------------------------

new_case du-fails
fill "$repo/server/target"
stub du "echo 'du: stub failure' >&2; exit 1"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "cannot measure"
expect_kept "$repo/server/target"

# du exits 0 but prints no size.
new_case du-prints-nothing
fill "$repo/server/target"
stub du "exit 0"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "cannot measure"
expect_kept "$repo/server/target"

new_case rm-fails
fill "$repo/server/target"
stub rm "echo 'rm: stub failure' >&2; exit 1"
run_clean SIDEREAL_TARGET_LIMIT_MIB=1 PATH="$bin:$PATH"
expect_status_nonzero
expect_err "may be partly deleted"

if [ "$failures" -ne 0 ]; then
  echo "target-clean-check: $failures failure(s)" >&2
  exit 1
fi
echo "target-clean-check: all cases passed"
