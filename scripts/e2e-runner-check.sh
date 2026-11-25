#!/usr/bin/env bash
# Checks how scripts/e2e.sh owns, restarts and removes its stack, with the
# real server, Vite and Chromium. web/e2e/runner-check.spec.ts holds the
# browser open while this check signals the runner.
#
#   bash scripts/e2e-runner-check.sh
#
# The database fixture must be up (`mise run db-up`). Each case starts a
# whole stack, so it takes a minute or two. Needs perl, node and curl. It
# starts the runner with SIGINT restored, because a background job of a
# non-interactive shell starts with SIGINT ignored.
set -euo pipefail
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"
# shellcheck source=scripts/e2e-lib.sh
. "$root/scripts/e2e-lib.sh"
e2e_ports "$root"

fx=$(mktemp -d "${TMPDIR:-/tmp}/e2e-runner-check.XXXXXX")
runner=
others=()
cleanup() {
  if [ -n "$runner" ]; then kill -TERM "$runner" 2>/dev/null || :; fi
  for pid in "${others[@]}"; do kill -KILL "$pid" 2>/dev/null || :; done
  rm -rf "$fx"
}
trap cleanup EXIT

failures=0
fail() {
  echo "FAIL [$label]: $*" >&2
  failures=$((failures + 1))
}

databases_before=$(sh server/scripts/db-fixture.sh e2e-db-list)

# start MODE [VAR=VALUE...]: starts the runner in the background as
# $runner, with its run folder under $fx/tmp and its output in $fx/log.
start() {
  local mode=$1
  shift
  rm -rf "$fx/tmp"
  mkdir "$fx/tmp"
  env TMPDIR="$fx/tmp" SIDEREAL_E2E_RUNNER_CHECK="$mode" "$@" \
    perl -e '$SIG{INT} = "DEFAULT"; exec @ARGV or die' sh scripts/e2e.sh >"$fx/log" 2>&1 &
  runner=$!
}

run_folder() {
  find "$fx/tmp" -mindepth 1 -maxdepth 1 -type d -name 'sidereal-e2e.*' | head -n 1
}

# wait_for PATH SECONDS: waits until PATH exists, while the runner runs.
wait_for() {
  local limit=$((SECONDS + $2))
  while [ ! -e "$1" ]; do
    if ! kill -0 "$runner" 2>/dev/null; then return 1; fi
    if [ "$SECONDS" -ge "$limit" ]; then return 1; fi
    sleep 0.1
  done
}

# finished SECONDS: waits for the runner to exit and sets $status.
finished() {
  local limit=$((SECONDS + $1))
  while alive "$runner" && [ "$SECONDS" -lt "$limit" ]; do sleep 0.1; done
  if alive "$runner"; then
    fail "the runner did not exit within $1 seconds"
    kill -KILL "$runner" 2>/dev/null || :
  fi
  status=0
  wait "$runner" || status=$?
  runner=
}

# held: waits for the runner to start its stack and the test to hold.
held() {
  local dir
  for _ in $(seq 6000); do
    dir=$(run_folder)
    [ -n "$dir" ] && break
    alive "$runner" || break
    sleep 0.1
  done
  [ -n "$dir" ] || { fail "the runner made no run folder"; return 1; }
  wait_for "$dir/holding" 600 || { fail "the browser test did not start holding"; return 1; }
  run=$dir
}

# removed: the run left no folder, database or listening port behind.
removed() {
  [ -z "$(run_folder)" ] || fail "the run folder $(run_folder) is still there"
  [ "$(sh server/scripts/db-fixture.sh e2e-db-list)" = "$databases_before" ] ||
    fail "a database the run made is still there"
  if port_busy "$e2e_server_port"; then fail "something still listens on the server port"; fi
  if port_busy "$e2e_web_port"; then fail "something still listens on the Vite port"; fi
}

label="SIGUSR1 restarts the server, with an inherited DATABASE_URL and STORAGE_ROOT"
start hold DATABASE_URL="$(sh server/scripts/db-fixture.sh url)" STORAGE_ROOT="$fx/elsewhere"
if held; then
  old=$(cat "$run/server.pid")
  if [ -r "/proc/$old/environ" ]; then
    tr '\0' '\n' <"/proc/$old/environ" | grep -q '^DATABASE_URL=.*/sidereal_test_e2e_[0-9a-f]*?' ||
      fail "the server's DATABASE_URL does not name a sidereal_test_e2e_ database"
  fi
  kill -USR1 "$runner"
  if wait_for "$run/ready-1" 120; then
    new=$(cat "$run/server.pid")
    [ "$new" != "$old" ] || fail "the server pid did not change"
    if alive "$old"; then fail "the first server still runs"; fi
    [ "$(ps -o ppid= -p "$new" | tr -d ' ')" = "$runner" ] || fail "the new server is not the runner's child"
    [ -e "$run/server-2.log" ] || fail "the new server has no log of its own"
    [ -z "$(find "$run/storage" \( -name 'server*' -o -name 'ready-*' -o -name holding \))" ] ||
      fail "the storage folder holds a process or log file of the run"
  else
    fail "the runner did not write ready-1"
  fi
  : >"$run/release"
fi
finished 120
[ "$status" -eq 0 ] || fail "the run exited with status $status"
[ ! -e "$fx/elsewhere" ] || fail "the run used the inherited STORAGE_ROOT"
removed

label="SIGINT to the runner alone stops Vite and the server"
start hold
if held; then
  port_busy "$e2e_web_port" || fail "Vite was not listening while the test held"
  server=$(cat "$run/server.pid")
  kill -INT "$runner"
  finished 60
  [ "$status" -ne 0 ] || fail "the run exited with status 0"
  if alive "$server"; then fail "the server still runs"; fi
  removed
else
  finished 60
fi

label="SIGINT during a restart"
start hold
if held; then
  old=$(cat "$run/server.pid")
  kill -USR1 "$runner"
  if wait_for "$run/server-2.log" 60; then
    [ ! -e "$run/ready-1" ] || fail "the restart ended before the check could interrupt it"
    new=$(cat "$run/server.pid")
    kill -INT "$runner"
    finished 60
    [ "$status" -ne 0 ] || fail "the run exited with status 0"
    if alive "$old" || alive "$new"; then fail "a server the run started still runs"; fi
    removed
  else
    fail "the runner did not start a second server"
    finished 60
  fi
else
  finished 60
fi

label="a failing browser test"
start fail
finished 600
[ "$status" -ne 0 ] || fail "the run exited with status 0"
grep -q "the runner check asked this test to fail" "$fx/log" || fail "the browser test did not run"
removed

label="another process on the server port"
: >"$fx/received"
node -e '
  const fs = require("node:fs");
  const [port, file] = process.argv.slice(1);
  require("node:net").createServer((socket) => socket.on("data", (data) => fs.appendFileSync(file, data)))
    .listen(Number(port), "127.0.0.1", () => fs.writeFileSync(file + ".ready", ""));
' "$e2e_server_port" "$fx/received" &
others+=($!)
disown
for _ in $(seq 50); do [ -e "$fx/received.ready" ] && break; sleep 0.1; done
start hold
finished 60
[ "$status" -ne 0 ] || fail "the run exited with status 0"
grep -q "already accepts connections on port $e2e_server_port" "$fx/log" || fail "the runner did not name the busy port"
[ -z "$(run_folder)" ] || fail "the runner made a run folder"
[ ! -s "$fx/received" ] || fail "the other process received bytes from the run"
grep -q "Running .* test" "$fx/log" && fail "Playwright started a browser test"

if [ "$failures" -gt 0 ]; then
  echo "e2e-runner-check: $failures failed; the last runner log follows" >&2
  cat "$fx/log" >&2
  exit 1
fi
echo "e2e-runner-check: all cases passed"
