#!/bin/sh
# End-to-end tests on a throwaway stack. Run through `mise run e2e`, or as
# `sh scripts/e2e.sh` without mise. The database fixture must already be up
# (`mise run db-up`). Arguments go to `playwright test`.
#
# Each run makes its own stack and removes it afterwards:
#   - a new database, sidereal_test_e2e_<random>, made through the
#     fixture's test role;
#   - a run folder from mktemp -d, holding an empty storage/ folder (the
#     server's STORAGE_ROOT), one log for each server start (server-1.log,
#     server-2.log, ...), server.pid and a ready-<n> file after each
#     restart;
#   - its own server and Vite, on ports derived from the checkout.
# It never reuses a running `mise run dev`, and ignores an inherited
# DATABASE_URL, STORAGE_ROOT and WEB_ROOT.
#
# A browser test restarts the server by sending SIGUSR1 to the process id
# in SIDEREAL_E2E_RUNNER_PID; it waits for ready-<n> in
# SIDEREAL_E2E_RUN_DIR, where <n> counts the restarts.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"
. "$root/scripts/e2e-lib.sh"

unset DATABASE_URL STORAGE_ROOT WEB_ROOT
e2e_ports "$root"
server_port=$e2e_server_port
web_port=$e2e_web_port

# Refuse before starting anything when something holds either port, so the
# run never talks to a server it did not start.
for port in "$server_port" "$web_port"; do
  if port_busy "$port"; then
    echo "e2e: something already accepts connections on port $port; stop it, then run again" >&2
    exit 1
  fi
done

# Build first, so compiling stays out of the readiness wait's limit.
cargo build -p sidereal-server --manifest-path server/Cargo.toml
(
  cd web
  pnpm install --frozen-lockfile --reporter=append-only
  # Playwright owns its own browser. The download is skipped when the
  # browser this version wants is already in the cache.
  pnpm exec playwright install chromium
)

run_dir=
db_name=
database_url=
server_pid=
playwright_pid=
starts=0
restarts=0
restart_requested=

# start_server: starts the next server as this script's own child, with
# its own log, and records its process id.
start_server() {
  starts=$((starts + 1))
  PORT=$server_port DATABASE_URL=$database_url STORAGE_ROOT=$run_dir/storage \
    server/target/debug/sidereal-server >"$run_dir/server-$starts.log" 2>&1 &
  server_pid=$!
  echo "$server_pid" >"$run_dir/server.pid"
  if ! wait_for_server "$server_pid" "$run_dir/server-$starts.log" "$server_port"; then
    show_log
    return 1
  fi
}

show_log() {
  if [ -f "$run_dir/server-$starts.log" ]; then
    echo "e2e: the last lines of server-$starts.log:" >&2
    tail -n 40 "$run_dir/server-$starts.log" >&2
  fi
}

# restart_server: stops the current server, then starts a new one on the
# same database and storage folder, and writes ready-<n> once it is ready.
restart_server() {
  echo "e2e: restarting the server" >&2
  stop_server "$server_pid" || return 1
  start_server || return 1
  restarts=$((restarts + 1))
  : >"$run_dir/ready-$restarts"
}

# cleanup: stops Playwright's process tree and the server, removes the run
# folder, then drops the database. Each step runs even when an earlier one
# failed; `left` names what is left behind.
left=
cleanup() {
  if [ -n "$playwright_pid" ] && ! stop_tree "$playwright_pid"; then
    left="$left; Playwright's processes (below pid $playwright_pid)"
  fi
  if [ -n "$server_pid" ] && ! stop_server "$server_pid"; then
    left="$left; the server (pid $server_pid)"
  fi
  if [ -n "$run_dir" ]; then
    rm -rf "$run_dir" 2>/dev/null || :
    if [ -e "$run_dir" ]; then left="$left; the run folder $run_dir"; fi
  fi
  if [ -n "$db_name" ] && ! sh server/scripts/db-fixture.sh e2e-db-drop "$db_name"; then
    left="$left; the database $db_name (mise run db-test-clean removes it)"
  fi
}

# finish STATUS: cleans up and exits. The status is the tests' result,
# except that a passing run whose cleanup failed exits 1.
finish() {
  status=$1
  # A second Ctrl-C must not stop the cleanup half-way.
  trap - EXIT
  trap '' INT TERM HUP USR1
  set +e
  cleanup
  if [ -n "$left" ]; then
    echo "e2e: cleanup failed; left behind${left#;}" >&2
    if [ "$status" -eq 0 ]; then status=1; fi
  fi
  exit "$status"
}

# A command that fails under `set -e` cleans up too.
trap 'finish $?' EXIT
trap 'finish 130' INT
trap 'finish 143' TERM HUP
trap 'restart_requested=1' USR1

run_dir=$(mktemp -d "${TMPDIR:-/tmp}/sidereal-e2e.XXXXXX") || {
  echo "e2e: cannot make the run folder" >&2
  finish 1
}
mkdir "$run_dir/storage" || finish 1
if ! db_name=$(sh server/scripts/db-fixture.sh e2e-db-create); then
  db_name=
  echo "e2e: cannot make the throwaway database; is the fixture up? Run mise run db-up" >&2
  finish 1
fi
database_url=$(sh server/scripts/db-fixture.sh e2e-db-url "$db_name")
echo "e2e: database $db_name, run folder $run_dir, server port $server_port, web port $web_port" >&2

start_server || finish 1

# Playwright runs as a background child that leads its own session, so a
# trapped signal can interrupt the wait for it and cleanup can stop its
# whole tree. Each branch backgrounds a simple command, so $! is the
# session leader.
export SIDEREAL_E2E_SERVER_PORT="$server_port" SIDEREAL_E2E_WEB_PORT="$web_port"
export SIDEREAL_E2E_RUNNER_PID="$$" SIDEREAL_E2E_RUN_DIR="$run_dir"
playwright='cd web && exec pnpm exec playwright test "$@"'
if command -v setsid >/dev/null 2>&1; then
  setsid sh -c "$playwright" playwright "$@" &
else
  # macOS has no setsid command, but has perl.
  perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid: $!\n"; exec @ARGV or die "exec: $!\n"' \
    sh -c "$playwright" playwright "$@" &
fi
playwright_pid=$!

# A trapped SIGUSR1 makes `wait` return early. Then the server restarts and
# the script waits again on the same Playwright child; only that child's
# exit ends the run.
while :; do
  if [ -n "$restart_requested" ]; then
    restart_requested=
    restart_server || {
      echo "e2e: the server did not come back after a restart" >&2
      finish 1
    }
  fi
  status=0
  wait "$playwright_pid" || status=$?
  if [ -n "$restart_requested" ] && alive "$playwright_pid"; then continue; fi
  break
done
playwright_pid=
if [ "$status" -ne 0 ]; then show_log; fi
finish "$status"
