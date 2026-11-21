#!/bin/sh
# Checks that the web shell's dev server forwards to the server that
# SIDEREAL_DEV_PROXY_TARGET names, as the end-to-end runner uses it. This
# is the web-shell scenario "The end-to-end runner names another server":
# with nothing on port 5000, it starts the server on 5901 against a
# throwaway database and Vite on 5902, then asks Vite for /healthz.
#
#   sh scripts/dev-proxy-check.sh
#
# The database fixture must be up (`mise run db-up`). Needs node and curl.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"
. "$root/scripts/e2e-lib.sh"

for port in 5000 5901 5902; do
  if port_busy "$port"; then
    echo "dev-proxy-check: something listens on port $port; the check needs it idle" >&2
    exit 1
  fi
done

cargo build -p sidereal-server --manifest-path server/Cargo.toml
(cd web && pnpm install --frozen-lockfile --reporter=append-only)

fx=
db_name=
server_pid=
vite_pid=
cleanup() {
  trap - EXIT
  trap '' INT TERM HUP
  set +e
  [ -z "$vite_pid" ] || stop_tree "$vite_pid"
  [ -z "$server_pid" ] || stop_server "$server_pid"
  [ -z "$fx" ] || rm -rf "$fx"
  [ -z "$db_name" ] || sh server/scripts/db-fixture.sh e2e-db-drop "$db_name"
}
trap 'status=$?; cleanup; exit "$status"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM HUP

fx=$(mktemp -d "${TMPDIR:-/tmp}/dev-proxy-check.XXXXXX")
db_name=$(sh server/scripts/db-fixture.sh e2e-db-create)
DATABASE_URL=$(sh server/scripts/db-fixture.sh e2e-db-url "$db_name") PORT=5901 \
  server/target/debug/sidereal-server >"$fx/server.log" 2>&1 &
server_pid=$!
wait_for_server "$server_pid" "$fx/server.log" 5901 || {
  cat "$fx/server.log" >&2
  exit 1
}

# Vite leads its own session, so cleanup stops pnpm and Vite together.
if command -v setsid >/dev/null 2>&1; then
  SIDEREAL_DEV_PROXY_TARGET=http://localhost:5901 \
    setsid sh -c 'cd web && exec pnpm dev --port 5902' >"$fx/vite.log" 2>&1 &
else
  SIDEREAL_DEV_PROXY_TARGET=http://localhost:5901 \
    perl -MPOSIX -e 'defined POSIX::setsid() or die "setsid: $!\n"; exec @ARGV or die "exec: $!\n"' \
    sh -c 'cd web && exec pnpm dev --port 5902' >"$fx/vite.log" 2>&1 &
fi
vite_pid=$!
tries=0
until port_busy 5902; do
  tries=$((tries + 1))
  if [ "$tries" -ge 600 ] || ! alive "$vite_pid"; then
    echo "dev-proxy-check: Vite did not listen on port 5902" >&2
    cat "$fx/vite.log" >&2
    exit 1
  fi
  sleep 0.1
done

body=$(curl -s http://localhost:5902/healthz)
if [ "$body" != '{"status":"ok"}' ]; then
  echo "dev-proxy-check: expected {\"status\":\"ok\"} from http://localhost:5902/healthz, got: $body" >&2
  exit 1
fi
echo "dev-proxy-check: http://localhost:5902/healthz answered $body through Vite"
