#!/bin/sh
# Check installation wiring without a Docker daemon or operator configuration.
set -eu
umask 077
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
fail() { echo "compose-config-check: FAILED: $*" >&2; exit 1; }
for tool in docker jq; do command -v "$tool" >/dev/null 2>&1 || fail "$tool is required"; done
for name in $(env | sed -n 's/^\([A-Za-z_][A-Za-z0-9_]*\)=.*/\1/p'); do
    case "$name" in COMPOSE_* | POSTGRES_* | PG* | SIDEREAL_* | DATABASE_URL | STORAGE_ROOT) unset "$name" ;; esac
done
[ ! -L "$root/.workspace" ] || fail 'scratch root is a symbolic link'
mkdir -p "$root/.workspace"
work=$(mktemp -d "$root/.workspace/compose-config-check.XXXXXX")
trap 'rm -rf "$work"' EXIT
trap 'exit 130' HUP INT TERM
cd "$root"
envfile="$work/settings.env"
url='postgres://fixture:p%40ss@external-db:5432/named?sslmode=verify-full&sslrootcert=/etc/sidereal/postgres/ca.crt'
settings() {
    printf '%s\n' "DATABASE_URL=$url" 'POSTGRES_PASSWORD=fixture' \
        "SIDEREAL_DATABASE_TLS_PATH=$work/certificates" \
        "SIDEREAL_STORAGE_PATH=$work/storage" "SIDEREAL_DATABASE_PATH=$work/database" >"$envfile"
}
compose() {
    docker compose --project-name sidereal-config-check --project-directory "$root" \
        --env-file "$envfile" "$@" config --format json
}
refuse() {
    diagnostic=$1
    shift
    if compose "$@" >"$work/refusal.log" 2>&1; then fail "accepted invalid configuration ($diagnostic)"; fi
    grep -Fq "$diagnostic" "$work/refusal.log" || fail "unexpected refusal ($diagnostic)"
}
settings
compose -f compose.yml >"$work/bundled.json"
# An external installation needs no bundled database password.
sed '/^POSTGRES_PASSWORD=/d' "$envfile" >"$work/external.env"
mv "$work/external.env" "$envfile"
compose -f compose.external-db.yml >"$work/external.json"
for entry in bundled external; do
    jq -S '.services.app | del(.depends_on, .environment.DATABASE_URL)' "$work/$entry.json" >"$work/$entry-app.json"
done
cmp "$work/bundled-app.json" "$work/external-app.json" || fail 'app settings differ between entries'
jq -e --arg url "$url" '
    (.services | keys) == ["app"] and (.volumes | keys) == ["storage"] and
    .services.app.environment.DATABASE_URL == $url and
    (.services.app | has("depends_on") | not)
' "$work/external.json" >/dev/null || fail 'external service, volumes, or URL changed'
for value in missing empty; do
    settings
    sed '/^DATABASE_URL=/d' "$envfile" >"$work/refusal.env"
    mv "$work/refusal.env" "$envfile"
    [ "$value" != empty ] || printf 'DATABASE_URL=\n' >>"$envfile"
    refuse DATABASE_URL -f compose.external-db.yml
done
settings
refuse 'neither an image nor a build' -f compose.external-db.yml -f compose.database-bind.yml
compose -f compose.external-db.yml -f compose.database-tls.yml >"$work/tls.json"
jq -e --arg path "$work/certificates" '
    [.services.app.volumes[] | select(.target == "/etc/sidereal/postgres")] |
    length == 1 and .[0].type == "bind" and .[0].source == $path and
    .[0].read_only == true and .[0].bind.create_host_path == false
' "$work/tls.json" >/dev/null || fail 'certificate mount contract changed'
for value in missing empty; do
    settings
    sed '/^SIDEREAL_DATABASE_TLS_PATH=/d' "$envfile" >"$work/refusal.env"
    mv "$work/refusal.env" "$envfile"
    [ "$value" != empty ] || printf 'SIDEREAL_DATABASE_TLS_PATH=\n' >>"$envfile"
    refuse SIDEREAL_DATABASE_TLS_PATH -f compose.external-db.yml -f compose.database-tls.yml
done
settings
compose -f compose.external-db.yml -f compose.storage-bind.yml -f compose.database-tls.yml >"$work/storage.json"
jq -e --arg path "$work/storage" '
    [.services.app.volumes[] | select(.target == "/var/lib/sidereal/storage")] |
    length == 1 and .[0].type == "bind" and .[0].source == $path and .[0].bind.create_host_path == false
' "$work/storage.json" >/dev/null || fail 'external storage override changed'
echo 'compose-config-check: passed (no daemon required)'
