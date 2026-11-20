#!/bin/sh
# Development servers. Runs without mise, so contributors on the
# hand-install route run the same commands as `mise run server`, `web`
# and `dev`.
#
#   sh scripts/dev.sh server   the server, against DATABASE_URL
#   sh scripts/dev.sh web      the web shell, after installing from the lockfile
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

case "${1:-}" in
  server)
    exec cargo run -p sidereal-server --manifest-path server/Cargo.toml
    ;;
  web)
    cd web
    pnpm install --frozen-lockfile --reporter=append-only
    exec pnpm dev
    ;;
  *)
    echo "usage: sh scripts/dev.sh [server|web]" >&2
    exit 2
    ;;
esac
