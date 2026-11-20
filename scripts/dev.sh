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

# Each process tree is stopped by walking `ps`, not by process group:
# dash turns job control off when no terminal is attached, so background
# jobs share this shell's group.
descendants() {
  ps -A -o pid= -o ppid= | awk -v root="$1" '
    { parent[$1] = $2 }
    END {
      found[root] = 1
      do {
        more = 0
        for (p in parent)
          if (!(p in found) && (parent[p] in found)) { found[p] = 1; more = 1 }
      } while (more)
      for (p in found) if (p != root) print p
    }'
}

alive() {
  for pid in "$@"; do kill -0 "$pid" 2>/dev/null && return 0; done
  return 1
}

# Stops each tree and waits up to 4 seconds for every process in it, so
# nothing outlives the script, such as Vite under pnpm.
stop_trees() {
  # List the whole tree before signalling, so no child is reparented away.
  pids=
  for root_pid in "$@"; do pids="$pids $root_pid $(descendants "$root_pid")"; done
  # shellcheck disable=SC2086
  kill -TERM $pids 2>/dev/null || :
  tries=0
  # shellcheck disable=SC2086
  while alive $pids && [ "$tries" -lt 40 ]; do
    sleep 0.1
    tries=$((tries + 1))
  done
  # shellcheck disable=SC2086
  kill -KILL $pids 2>/dev/null || :
  for root_pid in "$@"; do wait "$root_pid" 2>/dev/null || :; done
}

server_pid=
web_pid=

supervise() {
  trap 'trap - INT TERM HUP; stop_trees $server_pid $web_pid; exit 130' INT
  trap 'trap - INT TERM HUP; stop_trees $server_pid $web_pid; exit 143' TERM HUP
  sh "$root/scripts/dev.sh" server &
  server_pid=$!
  sh "$root/scripts/dev.sh" web &
  web_pid=$!
  # A one-second poll keeps the traps simple: a signal interrupts the
  # foreground sleep, and the trap then runs.
  while kill -0 "$server_pid" 2>/dev/null && kill -0 "$web_pid" 2>/dev/null; do
    sleep 1
  done
  if kill -0 "$server_pid" 2>/dev/null; then
    first=$web_pid other=$server_pid
  else
    first=$server_pid other=$web_pid
  fi
  status=0
  wait "$first" || status=$?
  trap - INT TERM HUP
  stop_trees "$other"
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
