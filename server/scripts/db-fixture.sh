#!/bin/sh
# The fixture is always selected from this checkout, never DATABASE_URL.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
key=$(printf '%s' "$root" | cksum | awk '{print $1}')
project="sidereal-db-$key"
SIDEREAL_DB_PORT=${SIDEREAL_DB_PORT:-$(expr 55000 + "$key" % 1000)}
export SIDEREAL_DB_PORT
case "$SIDEREAL_DB_PORT" in *[!0-9]*|'') echo 'SIDEREAL_DB_PORT must be a loopback port number' >&2; exit 1;; esac
[ "$SIDEREAL_DB_PORT" -gt 0 ] && [ "$SIDEREAL_DB_PORT" -le 65535 ] || exit 1
compose() {
    docker compose --project-name "$project" --env-file "$root/server/postgres-image.env" \
        -f "$root/server/tests/fixtures/compose.yml" "$@"
}
case "${1:-}" in
    up) compose up -d --wait --wait-timeout 90 ;;
    down) compose down ;;
    clean) compose down --volumes ;;
    url) printf 'postgresql://fixture_runtime:fixture_runtime@127.0.0.1:%s/fixture_runtime?sslmode=disable\n' "$SIDEREAL_DB_PORT" ;;
    test-url) printf 'postgresql://fixture_test:fixture_test@127.0.0.1:%s/fixture_control?sslmode=disable\n' "$SIDEREAL_DB_PORT" ;;
    demo-url) printf 'postgresql://fixture_demo:fixture_demo@127.0.0.1:%s/fixture_demo?sslmode=disable\n' "$SIDEREAL_DB_PORT" ;;
    *) echo 'Expected up, down, clean, url, test-url, or demo-url' >&2; exit 1 ;;
esac
