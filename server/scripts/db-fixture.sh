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
# Runs one SQL statement as the test role. On failure it prints the
# statement's SQLSTATE, or the whole error when there is none, such as when
# the fixture is not running.
test_sql() {
    if out=$(compose exec -T -e PGPASSWORD=fixture_test postgres \
        psql -X -q -h 127.0.0.1 -U fixture_test -d fixture_control \
        -v ON_ERROR_STOP=1 -v VERBOSITY=sqlstate -c "$1" 2>&1 >/dev/null); then
        return 0
    fi
    code=$(printf '%s\n' "$out" | sed -n 's/^ERROR: *\([0-9A-Z]\{5\}\)$/\1/p' | head -n 1)
    printf '%s\n' "${code:-$out}"
    return 1
}
# An end-to-end database name: the prefix, then random lowercase hex. The
# `sidereal_test_` prefix lets orphan-clean remove one a killed run left.
e2e_name() {
    case "${1:-}" in
        sidereal_test_e2e_|sidereal_test_e2e_*[!0-9a-f]*) ;;
        sidereal_test_e2e_*) return 0 ;;
    esac
    echo "Expected a database name that starts with sidereal_test_e2e_ and ends in lowercase hex, not '${1:-}'" >&2
    exit 1
}
# The drop policy of server/crates/core/src/database/test_support.rs: up to
# three rounds. Each tries a forced drop, and falls back to a plain drop when
# the forced one is refused for privilege (42501): the test role cannot stop
# an autovacuum worker, which a plain drop stops itself. A round ends the
# policy unless the refusal is 42501 or 55006 (another session is using it).
e2e_drop() {
    round=0
    while [ "$round" -lt 3 ]; do
        round=$((round + 1))
        code=$(test_sql "DROP DATABASE $1 WITH (FORCE)") && return 0
        if [ "$code" = 42501 ]; then
            code=$(test_sql "DROP DATABASE $1") && return 0
        fi
        case "$code" in 42501|55006) ;; *) break ;; esac
    done
    echo "Could not drop $1: $code" >&2
    return 1
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
    # The end-to-end runner's throwaway database, owned by the test role.
    # e2e-db-create prints the new database's name.
    e2e-db-create)
        name="sidereal_test_e2e_$(od -An -N8 -tx1 /dev/urandom | tr -d ' \n')"
        if ! code=$(test_sql "CREATE DATABASE $name"); then
            echo "Could not create $name: $code" >&2
            exit 1
        fi
        printf '%s\n' "$name"
        ;;
    # The test role's end-to-end databases, one name per line.
    e2e-db-list)
        compose exec -T -e PGPASSWORD=fixture_test postgres \
            psql -X -q -At -h 127.0.0.1 -U fixture_test -d fixture_control -v ON_ERROR_STOP=1 \
            -c "SELECT datname FROM pg_database WHERE starts_with(datname, 'sidereal_test_e2e_') ORDER BY datname"
        ;;
    e2e-db-url) e2e_name "${2:-}"; printf 'postgresql://fixture_test:fixture_test@127.0.0.1:%s/%s?sslmode=disable\n' "$SIDEREAL_DB_PORT" "$2" ;;
    e2e-db-drop) e2e_name "${2:-}"; e2e_drop "$2" ;;
    *) echo 'Expected up, down, stop, clean, url, test-url, orphan-clean, e2e-db-create, e2e-db-list, e2e-db-url NAME, or e2e-db-drop NAME' >&2; exit 1 ;;
esac
