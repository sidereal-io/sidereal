#!/usr/bin/env bash
# Tests the end-to-end runner's readiness wait, `wait_for_server` in
# scripts/e2e-lib.sh, with stub children instead of the real server. Each
# case runs under every shell here that the runner may use (dash, and bash
# in POSIX mode).
#
#   bash scripts/e2e-readiness.test.sh
#
# Needs node and curl, as the runner does. A recorder listens on the chosen
# port in the failing cases, so the test can tell that the wait sent no
# request there.
set -euo pipefail
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
lib="$root/scripts/e2e-lib.sh"

fx=$(mktemp -d "${TMPDIR:-/tmp}/e2e-readiness.XXXXXX")
pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill -KILL "$pid" 2>/dev/null || :; done
  rm -rf "$fx"
}
trap cleanup EXIT

free_port() {
  node -e 'const s = require("node:net").createServer().listen(0, "127.0.0.1", () => { console.log(s.address().port); s.close(); });'
}

# record PORT FILE: listens on PORT and appends one line to FILE for every
# connection, until the test ends.
record() {
  node -e '
    const fs = require("node:fs");
    const [port, file] = process.argv.slice(1);
    require("node:net").createServer((socket) => { fs.appendFileSync(file, "connection\n"); socket.destroy(); })
      .listen(Number(port), "127.0.0.1", () => fs.writeFileSync(file + ".ready", ""));
  ' "$1" "$2" &
  pids+=($!)
  # The test kills recorders at the end; bash need not report it.
  disown
  for _ in $(seq 50); do [ -e "$2.ready" ] && return 0; sleep 0.1; done
  echo "recorder did not start" >&2
  exit 1
}

# The stubs. Each one is the command a case starts as the child; `exec`
# makes the stub the child itself, so the case can stop it.
exits_quietly='echo "starting"; exit 3'
# shellcheck disable=SC2016
prints_port='exec node -e "
  console.log(\"sidereal-server listening on 0.0.0.0:\" + process.argv[1] + \" (stub)\");
  setTimeout(() => {}, 60000);
" "$STUB_PORT"'
# shellcheck disable=SC2016
serves_port='exec node -e "
  require(\"node:http\").createServer((req, res) => res.end(JSON.stringify({ status: \"ok\" })))
    .listen(Number(process.argv[1]), \"127.0.0.1\", () => console.log(\"sidereal-server listening on 0.0.0.0:\" + process.argv[1] + \" (core abi 1)\"));
" "$STUB_PORT"'

# readiness SHELL STUB STUB_PORT PORT LIMIT: starts STUB as a child of SHELL
# with its output in a log, and runs the wait on it for PORT. Prints the
# wait's status.
readiness() {
  local log="$fx/log.$RANDOM"
  # shellcheck disable=SC2086
  STUB_PORT=$3 $1 -c '
    . "$1"
    sh -c "$2" >"$3" 2>&1 &
    child=$!
    status=0
    wait_for_server "$child" "$3" "$4" "$5" 2>/dev/null || status=$?
    kill -KILL "$child" 2>/dev/null || :
    echo "$status"
  ' readiness "$lib" "$2" "$log" "$4" "$5"
}

failures=0
fail() {
  echo "FAIL [$label]: $*" >&2
  failures=$((failures + 1))
}

shells=()
command -v dash >/dev/null 2>&1 && shells+=(dash)
shells+=("bash --posix")

for shell in "${shells[@]}"; do
  label="$shell: a child that exits without the listening line"
  port=$(free_port)
  : >"$fx/requests"
  rm -f "$fx/requests.ready"
  record "$port" "$fx/requests"
  started=$SECONDS
  status=$(readiness "$shell" "$exits_quietly" "$port" "$port" 30)
  [ "$status" != 0 ] || fail "the wait succeeded"
  # The wait notices the exit itself, long before its limit.
  [ $((SECONDS - started)) -lt 10 ] || fail "the wait took $((SECONDS - started)) seconds to notice the exit"
  [ ! -s "$fx/requests" ] || fail "the wait connected to the chosen port"

  for other in another longer; do
    label="$shell: a running child that prints $other port"
    port=$(free_port)
    if [ "$other" = another ]; then printed=$(free_port); else printed="${port}0"; fi
    : >"$fx/requests"
    rm -f "$fx/requests.ready"
    record "$port" "$fx/requests"
    status=$(readiness "$shell" "$prints_port" "$printed" "$port" 2)
    [ "$status" != 0 ] || fail "the wait succeeded"
    [ ! -s "$fx/requests" ] || fail "the wait connected to the chosen port"
  done

  label="$shell: a running child that prints the chosen port"
  port=$(free_port)
  status=$(readiness "$shell" "$serves_port" "$port" "$port" 20)
  [ "$status" = 0 ] || fail "the wait failed with status $status"
done

if [ "$failures" -gt 0 ]; then
  echo "e2e-readiness: $failures failed" >&2
  exit 1
fi
echo "e2e-readiness: all cases passed"
