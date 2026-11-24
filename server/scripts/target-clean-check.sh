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

if [ "$failures" -ne 0 ]; then
  echo "target-clean-check: $failures failure(s)" >&2
  exit 1
fi
echo "target-clean-check: all cases passed"
