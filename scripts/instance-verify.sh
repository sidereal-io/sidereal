#!/bin/sh
# Build the app image, start a disposable instance from the root compose.yml,
# check it, and remove it. Run it through `just instance-verify`.
#
# The run uses its own Compose project, image, password, and host port. It
# never reads the root .env, and it removes nothing that another Compose
# project owns. Set SIDEREAL_VERIFY_PORT when the derived host port is taken.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
work="$root/.workspace/instance-verify"
lock="$work/lock"
record="$work/created"
# A SHA-256 prefix names the project, so two checkout paths do not share one
# in practice. The shorter checksum only picks the default host port.
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi; }
project="sidereal-verify-$(printf '%s' "$root" | sha256 | cut -c1-16)"
key=$(printf '%s' "$root" | cksum | awk '{print $1}')
port=${SIDEREAL_VERIFY_PORT:-$(expr 56000 + "$key" % 1000)}
deadline=180

held=0
started=0
up=''

say() { echo "instance-verify: $*"; }
fail() { echo "instance-verify: FAILED: $*" >&2; exit 1; }

# With run=exec, Compose replaces the calling subshell, so `$!` names Compose
# itself and a kill reaches it.
compose() {
    ${run:-} docker compose --project-name "$project" --project-directory "$root" \
        --env-file "$work/empty.env" -f "$root/compose.yml" "$@"
}

# Every container and volume that carries this run's project label.
owned() {
    docker ps -a -q --filter "label=com.docker.compose.project=$project"
    docker volume ls -q --filter "label=com.docker.compose.project=$project"
}

cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ -n "$up" ]; then
        kill "$up" 2>/dev/null || true
        wait "$up" 2>/dev/null || true
    fi
    if [ "$started" = 1 ]; then
        if compose down --volumes --rmi local --remove-orphans --timeout 10 >"$work/down.log" 2>&1 \
            && [ -z "$(owned)" ]; then
            rm -f "$record"
        else
            echo "instance-verify: cleanup of project $project failed; see $work/down.log" >&2
            status=1
        fi
    fi
    [ "$held" = 0 ] || rm -rf "$lock"
    exit "$status"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

# --- One run per checkout. Creating a folder is atomic.
mkdir -p "$work"
if ! mkdir "$lock" 2>/dev/null; then
    pid=$(cat "$lock/pid" 2>/dev/null || true)
    if [ -z "$pid" ] || kill -0 "$pid" 2>/dev/null; then
        fail "a run is in progress in this checkout (process ${pid:-unknown}). If none is, remove $lock"
    fi
    # The run that held the lock died. One run at a time may replace the lock,
    # and it checks the process ID again once it is alone.
    mkdir "$work/reclaim" 2>/dev/null \
        || fail "a run is in progress in this checkout. If none is, remove $work/reclaim"
    pid=$(cat "$lock/pid" 2>/dev/null || true)
    if [ -z "$pid" ] || kill -0 "$pid" 2>/dev/null; then
        rmdir "$work/reclaim"
        fail "a run is in progress in this checkout (process ${pid:-unknown})"
    fi
    rm -rf "$lock"
    if ! mkdir "$lock" 2>/dev/null; then
        rmdir "$work/reclaim"
        fail 'a run is in progress in this checkout'
    fi
    echo $$ >"$lock/pid"
    rmdir "$work/reclaim"
else
    echo $$ >"$lock/pid"
fi
held=1

# --- Docker and Compose.
command -v docker >/dev/null 2>&1 || fail 'Docker is not installed. Install Docker Engine or Docker Desktop'
command -v curl >/dev/null 2>&1 || fail 'curl is not installed'
docker info >/dev/null 2>&1 || fail 'cannot reach the Docker daemon. Start Docker, then run this again'
version=$(docker compose version --short 2>/dev/null) || fail 'Docker Compose 2.20 or newer is required'
version=${version#v}
major=${version%%.*}
minor=${version#*.}
minor=${minor%%.*}
case "$major$minor" in *[!0-9]* | '') fail "Docker Compose 2.20 or newer is required; found $version" ;; esac
if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 20 ]; }; then
    fail "Docker Compose 2.20 or newer is required; found $version"
fi
case "$port" in *[!0-9]* | '') fail 'SIDEREAL_VERIFY_PORT must be a port number' ;; esac
[ "$port" -gt 0 ] && [ "$port" -le 65535 ] || fail 'SIDEREAL_VERIFY_PORT must be a port number'

# --- Settings. Exported values outrank the caller's shell, and the empty env
# file keeps Compose from reading the root .env.
new_password() { od -An -N32 -tx1 /dev/urandom | tr -d ' \n'; }
password=$(new_password)
: >"$work/empty.env"
export POSTGRES_PASSWORD="$password" SIDEREAL_PORT="$port"

# --- Ownership. The name alone proves nothing: another checkout could still
# hold it. Leftovers are ours only when this checkout recorded them.
if [ -n "$(owned)" ]; then
    if [ "$(cat "$record" 2>/dev/null || true)" != "$project" ]; then
        fail "Compose project $project already owns a container or volume that this checkout did not create. Nothing was removed"
    fi
    say "removing what an earlier run in this checkout left behind"
    compose down --volumes --rmi local --remove-orphans --timeout 10 >"$work/down.log" 2>&1 \
        || fail "could not remove the earlier run; see $work/down.log"
fi
printf '%s\n' "$project" >"$record"
started=1

# --- Checks that need no build.
say 'checking that Compose refuses a missing password'
for value in unset empty; do
    if [ "$value" = unset ]; then
        refusal=$(unset POSTGRES_PASSWORD && compose up -d --no-build 2>&1) && refused=0 || refused=1
    else
        refusal=$(POSTGRES_PASSWORD='' compose up -d --no-build 2>&1) && refused=0 || refused=1
    fi
    [ "$refused" = 1 ] || fail "Compose started with POSTGRES_PASSWORD $value"
    case "$refusal" in *POSTGRES_PASSWORD*) ;; *) fail "the refusal does not name POSTGRES_PASSWORD: $refusal" ;; esac
    [ -z "$(owned)" ] || fail "Compose created something with POSTGRES_PASSWORD $value"
done

say 'checking the PostgreSQL image pin'
pin=$(sed -n 's/^POSTGRES_IMAGE=//p' "$root/server/postgres-image.env")
[ -n "$pin" ] || fail 'server/postgres-image.env holds no POSTGRES_IMAGE'
compose config --images | grep -Fxq -- "$pin" \
    || fail "the db image in compose.yml differs from the pin in server/postgres-image.env ($pin)"

# --- Build, then start. The script measures the deadline itself: Compose's
# own wait timeout does not cover a dependency on every release.
say "building the app image for project $project"
compose build >"$work/build.log" 2>&1 || {
    tail -n 40 "$work/build.log" >&2
    fail "the image build failed; see $work/build.log"
}

show_state() {
    compose ps -a >&2 2>/dev/null || true
    compose logs --no-color --tail 40 >&2 2>/dev/null || true
}

# start <label>: `docker compose up --wait`, stopped after $deadline seconds.
start() {
    say "starting the instance ($1, at most $deadline seconds)"
    (run=exec && compose up -d --no-build --wait) >"$work/up.log" 2>&1 &
    up=$!
    waited=0
    while kill -0 "$up" 2>/dev/null; do
        if [ "$waited" -ge "$deadline" ]; then
            kill "$up" 2>/dev/null || true
            wait "$up" 2>/dev/null || true
            up=''
            show_state
            fail "the instance was not healthy within $deadline seconds ($1)"
        fi
        sleep 1
        waited=$((waited + 1))
    done
    if wait "$up"; then
        up=''
    else
        up=''
        tail -n 20 "$work/up.log" >&2
        show_state
        fail "the instance did not start ($1)"
    fi
}

# fetch <path>: the body, retried a fixed number of times.
fetch() {
    tries=0
    until body=$(curl -fsS --max-time 5 "http://127.0.0.1:$port$1" 2>/dev/null); do
        tries=$((tries + 1))
        [ "$tries" -lt 10 ] || return 1
        sleep 1
    done
    printf '%s' "$body"
}

migrations() {
    compose exec -T db psql -U sidereal -d sidereal -v ON_ERROR_STOP=1 -Atc \
        "SELECT version, encode(checksum, 'hex'), installed_on FROM _sqlx_migrations ORDER BY version"
}

# no_secret <text>...: fail when the logs of either service hold one of them.
no_secret() {
    logs=$(compose logs --no-color 2>&1)
    for secret in "$@"; do
        case "$logs" in *"$secret"*) fail 'the container logs hold a database password or connection URL' ;; esac
    done
}

start 'first start'

# --- Instance checks. The mount check runs first, before any `down`: when it
# fails, the exit trap's `down --volumes` still removes an unnamed volume.
say 'checking the database volume'
db=$(compose ps -q db)
mounts=$(docker inspect --format '{{range .Mounts}}{{.Type}} {{.Name}} {{.Destination}}{{"\n"}}{{end}}' "$db")
[ "$(printf '%s\n' "$mounts" | sed '/^$/d')" = "volume ${project}_database /var/lib/postgresql" ] \
    || fail "the db container does not keep its data in the named volume ${project}_database only: $mounts"

say 'checking the home page and the health probes'
app=$(compose ps -q app)
page=$(fetch /) || fail 'the home page did not load'
[ "$page" = "$(compose exec -T app sh -c 'cat "$WEB_ROOT/index.html"')" ] || fail 'the home page is not index.html'
curl -fsSI --max-time 5 "http://127.0.0.1:$port/" | grep -iq '^content-type: text/html' \
    || fail 'the home page is not text/html'
[ "$(fetch /healthz)" = '{"status":"ok"}' ] || fail '/healthz did not return {"status":"ok"}'
[ "$(fetch /readyz)" = '{"status":"ready"}' ] || fail '/readyz did not return {"status":"ready"}'

say 'checking the app user, the database port, and the storage volume'
[ "$(compose exec -T app id -u)" = 10001 ] || fail 'the app does not run as user ID 10001'
[ "$(docker inspect --format '{{json .HostConfig.PortBindings}}' "$db")" = '{}' ] \
    || fail 'the db container publishes a host port'
compose exec -T app sh -c 'touch /var/lib/sidereal/storage/probe && rm /var/lib/sidereal/storage/probe' \
    || fail 'the app user cannot write to the storage volume'
docker inspect --format '{{range .Mounts}}{{.Type}} {{.Name}} {{.Destination}}{{"\n"}}{{end}}' "$app" \
    | grep -Fxq "volume ${project}_storage /var/lib/sidereal/storage" \
    || fail 'the storage folder is not the named storage volume'

say 'checking the logs after a successful start'
no_secret "$password" 'postgres://'

# --- Retention: the migration records survive `docker compose down`.
say 'checking that the database survives new containers'
before=$(migrations)
[ -n "$before" ] || fail 'the database holds no migration record'
compose down --timeout 10 >"$work/down.log" 2>&1 || fail "docker compose down failed; see $work/down.log"
docker volume inspect "${project}_database" >/dev/null 2>&1 || fail 'docker compose down removed the database volume'
start 'second start'
[ "$(migrations)" = "$before" ] || fail 'the migration records changed across docker compose down'
[ "$(fetch /readyz)" = '{"status":"ready"}' ] || fail '/readyz did not return {"status":"ready"} after the second start'

# --- A failed database login must not print either password.
say 'checking the logs after a failed database login'
other=$(new_password)
POSTGRES_PASSWORD="$other" compose up -d --no-build --no-deps --force-recreate app >"$work/up.log" 2>&1 \
    || fail "could not start the app with another password; see $work/up.log"
app=$(compose ps -a -q app)
tries=0
# The app exits, and its restart policy starts it again.
until [ "$(docker inspect --format '{{if or (gt .RestartCount 0) (and (eq .State.Status "exited") (ne .State.ExitCode 0))}}failed{{end}}' "$app")" = failed ]; do
    tries=$((tries + 1))
    if [ "$tries" -ge 90 ]; then
        show_state
        fail 'the app did not exit with the wrong database password'
    fi
    sleep 1
done
no_secret "$password" "$other" 'postgres://'

say "passed. Removing project $project"
