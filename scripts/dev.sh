#!/bin/sh
# Development servers. Runs without mise, so contributors on the
# hand-install route run the same commands as `mise run server`, `web`
# and `dev`.
#
#   sh scripts/dev.sh server   the server, against DATABASE_URL
#   sh scripts/dev.sh web      the web shell, after installing from the lockfile
#   sh scripts/dev.sh          both; when either exits, the other stops too
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

# Each worker leads its own session, and so its own process group, which
# lasts while any process the worker started is alive, even after the
# worker itself has exited. Stopping the group therefore reaches children
# that were reparented away from the worker. Job control (`set -m`) cannot
# give each worker a group: dash turns it off when no terminal is attached.

alive() {
  for id in "$@"; do kill -0 "$id" 2>/dev/null && return 0; done
  return 1
}

server_pid=
web_pid=
events=

# Stops both workers' groups and waits up to 4 seconds for every process
# in them, so nothing outlives the script, such as Vite under pnpm.
stop_workers() {
  groups=
  for pid in $server_pid $web_pid; do groups="$groups -$pid"; done
  if [ -n "$groups" ]; then
    # shellcheck disable=SC2086
    kill -TERM $groups 2>/dev/null || :
    tries=0
    # shellcheck disable=SC2086
    while alive $groups && [ "$tries" -lt 40 ]; do
      sleep 0.1
      tries=$((tries + 1))
    done
    # shellcheck disable=SC2086
    kill -KILL $groups 2>/dev/null || :
    for pid in $server_pid $web_pid; do wait "$pid" 2>/dev/null || :; done
  fi
  rm -f "$events"
}

# Runs one worker in its own session. When the worker exits, the wrapper
# appends "<worker> <status>" to the events file, so the order in which
# the workers stopped is kept even when both stop between two polls.
# Each branch backgrounds a simple command, so $! is the session leader,
# whose PID is the group's ID.
start_worker() {
  wrapper='sh "$0" "$1"; echo "$1 $?" >>"$2"'
  if command -v setsid >/dev/null 2>&1; then
    setsid sh -c "$wrapper" "$root/scripts/dev.sh" "$1" "$events" &
  else
    # macOS has no setsid command, but has perl.
    perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid: $!\n"; exec @ARGV or die "exec: $!\n"' \
      sh -c "$wrapper" "$root/scripts/dev.sh" "$1" "$events" &
  fi
}

supervise() {
  command -v setsid >/dev/null 2>&1 || command -v perl >/dev/null 2>&1 || {
    echo "dev.sh: needs setsid or perl to run both workers" >&2
    exit 1
  }
  trap 'trap - INT TERM HUP; stop_workers; exit 130' INT
  trap 'trap - INT TERM HUP; stop_workers; exit 143' TERM HUP
  events=$(mktemp "${TMPDIR:-/tmp}/sidereal-dev.XXXXXX")
  start_worker server
  server_pid=$!
  start_worker web
  web_pid=$!
  # A one-second poll keeps the traps simple: a signal interrupts the
  # foreground sleep, and the trap then runs.
  while [ ! -s "$events" ]; do
    sleep 1
  done
  # The first worker's status, unless it is 0 and the other worker had
  # also stopped on its own and failed: a failure never comes out as 0.
  # shellcheck disable=SC2046
  set -- $(cat "$events")
  status=$2
  if [ "$status" -eq 0 ] && [ $# -ge 4 ]; then status=$4; fi
  trap - INT TERM HUP
  stop_workers
  exit "$status"
}

case "${1:-}" in
  server)
    exec cargo run -p sidereal-server --manifest-path server/Cargo.toml
    ;;
  web)
    cd web
    pnpm install --frozen-lockfile --reporter=append-only
    exec pnpm dev
    ;;
  '')
    supervise
    ;;
  *)
    echo "usage: sh scripts/dev.sh [server|web]" >&2
    exit 2
    ;;
esac
