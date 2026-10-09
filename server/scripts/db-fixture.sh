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
    up) sh "$root/server/scripts/db-tls.sh"; compose up -d --wait --wait-timeout 90 ;;
    down) compose down ;;
    stop) compose stop postgres ;;
    orphan-clean)
        compose exec -T postgres psql -U fixture_admin -d fixture_control -v ON_ERROR_STOP=1 <<'SQL'
SELECT format('DROP DATABASE %I WITH (FORCE)', datname) FROM pg_database WHERE starts_with(datname, 'sidereal_test_') AND datdba = (SELECT oid FROM pg_roles WHERE rolname = 'fixture_test');
\gexec
SQL
        ;;
    clean) compose down --volumes ;;
    url) printf 'postgresql://fixture_runtime:fixture_runtime@127.0.0.1:%s/fixture_runtime?sslmode=disable\n' "$SIDEREAL_DB_PORT" ;;
    test-url) printf 'postgresql://fixture_test:fixture_test@127.0.0.1:%s/fixture_control?sslmode=disable\n' "$SIDEREAL_DB_PORT" ;;
    *) echo 'Expected up, down, stop, clean, url, test-url, or orphan-clean' >&2; exit 1 ;;
esac
