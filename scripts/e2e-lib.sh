# Shell functions for scripts/e2e.sh. Source this file; do not run it.
# scripts/e2e-readiness.test.sh tests the readiness wait on its own.

# e2e_ports ROOT: sets e2e_server_port and e2e_web_port from the checkout's
# path, the way server/scripts/db-fixture.sh picks the database port, so
# neither is 5000 or 5173 and two checkouts do not collide.
e2e_ports() {
  _ep_key=$(printf '%s' "$1" | cksum | awk '{print $1}')
  e2e_server_port=$((56000 + _ep_key % 1000))
  e2e_web_port=$((57000 + _ep_key % 1000))
}

# alive PID: the process runs, and is not a zombie that its parent has not
# reaped yet.
alive() {
  kill -0 "$1" 2>/dev/null || return 1
  case "$(ps -o stat= -p "$1" 2>/dev/null)" in
    Z* | '') return 1 ;;
  esac
}

# port_busy PORT: something accepts TCP connections on PORT at 127.0.0.1 or
# [::1]. It opens a connection and closes it at once, sending no bytes, so
# whatever listens there gets no HTTP request.
port_busy() {
  node -e '
    const net = require("node:net");
    const port = Number(process.argv[1]);
    let left = 2;
    for (const host of ["127.0.0.1", "::1"]) {
      const socket = net.connect({ host, port });
      socket.setTimeout(1000);
      socket.on("connect", () => process.exit(0));
      const done = () => { socket.destroy(); if (--left === 0) process.exit(1); };
      socket.on("error", done);
      socket.on("timeout", done);
    }
  ' "$1"
}

# wait_for_server PID LOG PORT [LIMIT]
#
# Waits until the child PID has written a line that starts with
# `sidereal-server listening on` and names PORT to LOG, and is still
# running. Only then does it ask the server's /healthz on PORT. Another
# process could hold PORT and answer first, because the server prepares
# its database before it binds; the listening line is the evidence that
# this child bound it.
#
# Fails when the child exits first, and after LIMIT seconds (300 by
# default), which is as long as web/playwright.config.ts once waited for
# the server. Sends no request to PORT unless the listening line came.
wait_for_server() {
  _wfs_pid=$1
  _wfs_log=$2
  _wfs_port=$3
  _wfs_deadline=$(($(date +%s) + ${4:-300}))
  while :; do
    if ! alive "$_wfs_pid"; then
      echo "e2e: the server (pid $_wfs_pid) exited before it said it listens on port $_wfs_port" >&2
      return 1
    fi
    if grep -Eq "^sidereal-server listening on [^ ]*:$_wfs_port( |\$)" "$_wfs_log" 2>/dev/null; then
      break
    fi
    if [ "$(date +%s)" -ge "$_wfs_deadline" ]; then
      echo "e2e: the server (pid $_wfs_pid) did not say it listens on port $_wfs_port in time" >&2
      return 1
    fi
    sleep 0.2
  done
  while :; do
    if ! alive "$_wfs_pid"; then
      echo "e2e: the server (pid $_wfs_pid) exited after it said it listens" >&2
      return 1
    fi
    if curl -fsS --max-time 2 -o /dev/null "http://127.0.0.1:$_wfs_port/healthz" 2>/dev/null; then
      return 0
    fi
    if [ "$(date +%s)" -ge "$_wfs_deadline" ]; then
      echo "e2e: the server (pid $_wfs_pid) did not answer /healthz on port $_wfs_port in time" >&2
      return 1
    fi
    sleep 0.2
  done
}

# descendants PID: every process below PID, children first.
descendants() {
  ps -A -o pid= -o ppid= | awk -v root="$1" '
    { parent[$1] = $2 }
    END {
      found[root] = 1
      do {
        more = 0
        for (pid in parent)
          if (!(pid in found) && (parent[pid] in found)) { found[pid] = 1; more = 1; print pid }
      } while (more)
    }'
}

# stop_tree PID: stops the process group that PID leads and every process
# below PID, then reaps PID. Playwright starts Vite and Chromium in groups
# of their own, so the list of descendants is taken before any of them is
# signalled and reparented. Waits up to 4 seconds, as scripts/dev.sh does,
# before it kills what is left. Fails when anything is still running.
stop_tree() {
  _st_pids="$1 $(descendants "$1" | tr '\n' ' ')"
  # shellcheck disable=SC2086
  kill -TERM "-$1" $_st_pids 2>/dev/null || :
  _st_tries=0
  while [ "$_st_tries" -lt 40 ]; do
    _st_left=
    for _st_pid in $_st_pids; do alive "$_st_pid" && _st_left="$_st_left $_st_pid"; done
    [ -n "$_st_left" ] || break
    sleep 0.1
    _st_tries=$((_st_tries + 1))
  done
  # shellcheck disable=SC2086
  [ -z "$_st_left" ] || kill -KILL "-$1" $_st_left 2>/dev/null || :
  wait "$1" 2>/dev/null || :
  sleep 0.1
  for _st_pid in $_st_pids; do
    alive "$_st_pid" && return 1
  done
  return 0
}

# stop_server PID: asks the server to stop with SIGTERM, waits up to 10
# seconds for its graceful shutdown, then kills it, and reaps it.
stop_server() {
  kill -TERM "$1" 2>/dev/null || :
  _ss_tries=0
  while alive "$1" && [ "$_ss_tries" -lt 100 ]; do
    sleep 0.1
    _ss_tries=$((_ss_tries + 1))
  done
  alive "$1" && kill -KILL "$1" 2>/dev/null
  wait "$1" 2>/dev/null || :
  sleep 0.1
  ! alive "$1"
}
