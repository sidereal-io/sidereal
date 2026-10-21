#!/bin/sh
# Build the app image, start disposable instances from the root Compose files,
# check them, and remove them. Run it through `just instance-verify`.
#
# The run uses its own Compose project, image, password, host port, and bind
# directories under .workspace/instance-verify/binds. It never reads the root
# .env or the caller's mount settings, and it removes nothing that another
# Compose project owns. Set SIDEREAL_VERIFY_PORT when the derived port is taken.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
work="$root/.workspace/instance-verify"
lock="$work/lock"
record="$work/created"
envfile="$work/instance.env"
# This run's dedicated bind directories. Helper containers mount only these.
binds="$work/binds"
# A SHA-256 prefix names the project, so two checkout paths do not share one
# in practice. The shorter checksum only picks the default host port.
sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi; }
project="sidereal-verify-$(printf '%s' "$root" | sha256 | cut -c1-16)"
key=$(printf '%s' "$root" | cksum | awk '{print $1}')
port=${SIDEREAL_VERIFY_PORT:-$(expr 56000 + "$key" % 1000)}
deadline=180
storage=/var/lib/sidereal/storage
marker=.sidereal-storage.json

# The caller's mount selection and Compose settings never reach this run.
unset COMPOSE_FILE COMPOSE_PATH_SEPARATOR COMPOSE_ENV_FILES COMPOSE_PROFILES \
    SIDEREAL_STORAGE_PATH SIDEREAL_DATABASE_PATH STORAGE_ROOT POSTGRES_PASSWORD SIDEREAL_PORT
# Compose resolves COMPOSE_FILE from the working directory, as the guide does.
cd "$root"

held=0
started=0
up=''
pin=''

say() { echo "instance-verify: $*"; }
fail() { echo "instance-verify: FAILED: $*" >&2; exit 1; }

# With run=exec, Compose replaces the calling subshell, so `$!` names Compose
# itself and a kill reaches it. Settings come only from this run's env file.
compose() {
    ${run:-} docker compose --project-name "$project" --project-directory "$root" \
        --env-file "$envfile" "$@"
}

# settings <mode> [storage path] [database path]: write this run's .env.
# Modes: volumes, binds, no-password, empty-password. Every mode names its
# Compose files, so a compose.override.yml in the checkout is never loaded.
settings() {
    case "$1" in
        no-password) printf 'COMPOSE_FILE=compose.yml\n' >"$envfile" ;;
        empty-password) printf 'COMPOSE_FILE=compose.yml\nPOSTGRES_PASSWORD=\n' >"$envfile" ;;
        volumes) printf 'COMPOSE_FILE=compose.yml\nPOSTGRES_PASSWORD=%s\nSIDEREAL_PORT=%s\n' "$password" "$port" >"$envfile" ;;
        binds)
            printf '%s\n' "POSTGRES_PASSWORD=$password" "SIDEREAL_PORT=$port" \
                'COMPOSE_FILE=compose.yml:compose.storage-bind.yml:compose.database-bind.yml' \
                "SIDEREAL_STORAGE_PATH=${2:-$binds/storage}" \
                "SIDEREAL_DATABASE_PATH=${3:-$binds/database}" >"$envfile"
            ;;
    esac
}

# Every container and volume that carries this run's project label.
owned() {
    docker ps -a -q --filter "label=com.docker.compose.project=$project"
    docker volume ls -q --filter "label=com.docker.compose.project=$project"
}

# helper <dir> <command>...: run a command as root in a throwaway container
# that mounts only <dir>, one of this run's bind directories, at /target.
helper() {
    dir=$1
    shift
    case "$dir" in
        "$binds/storage" | "$binds/database" | "$binds/empty-storage") ;;
        *) fail "refusing to mount $dir into a helper container" ;;
    esac
    [ -d "$dir" ] && [ ! -L "$dir" ] || fail "$dir is not a plain directory"
    entrypoint=$1
    shift
    docker run --rm --network none --user 0:0 --cap-drop ALL \
        --cap-add CHOWN --cap-add DAC_OVERRIDE --cap-add FOWNER \
        --security-opt no-new-privileges:true --entrypoint "$entrypoint" \
        -v "$dir:/target" "$pin" "$@" >/dev/null 2>&1
}

# Remove this run's bind directories, including container-owned data inside
# them. `find -delete` never follows a symbolic link out of /target.
remove_binds() {
    [ -e "$binds" ] || return 0
    [ ! -L "$binds" ] || fail "$binds is a symbolic link. Nothing was removed"
    for dir in "$binds/storage" "$binds/database" "$binds/empty-storage"; do
        [ -e "$dir" ] || continue
        helper "$dir" find /target -mindepth 1 -delete || return 1
        rmdir "$dir" || return 1
    done
    rmdir "$binds"
}

cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ -n "$up" ]; then
        kill "$up" 2>/dev/null || true
        wait "$up" 2>/dev/null || true
    fi
    if [ "$started" = 1 ]; then
        settings binds
        if compose down --volumes --rmi local --remove-orphans --timeout 10 >"$work/down.log" 2>&1 \
            && [ -z "$(owned)" ] && remove_binds; then
            rm -f "$record" "$envfile"
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
# Bind specifications and COMPOSE_FILE cannot carry these characters.
case "$root" in *:* | *,*) fail "the checkout path must not contain ':' or ','" ;; esac

new_password() { od -An -N32 -tx1 /dev/urandom | tr -d ' \n'; }
password=$(new_password)
pin=$(sed -n 's/^POSTGRES_IMAGE=//p' "$root/server/postgres-image.env")
[ -n "$pin" ] || fail 'server/postgres-image.env holds no POSTGRES_IMAGE'

# --- Ownership. The name alone proves nothing: another checkout could still
# hold it. Leftovers are ours only when this checkout recorded them.
settings binds
if [ -n "$(owned)" ]; then
    if [ "$(cat "$record" 2>/dev/null || true)" != "$project" ]; then
        fail "Compose project $project already owns a container or volume that this checkout did not create. Nothing was removed"
    fi
    say "removing what an earlier run in this checkout left behind"
    compose down --volumes --rmi local --remove-orphans --timeout 10 >"$work/down.log" 2>&1 \
        || fail "could not remove the earlier run; see $work/down.log"
fi
# Bind directories hold container-owned data. Remove them only when this
# checkout's record names this project, before the record is replaced.
if [ -e "$binds" ] && [ "$(cat "$record" 2>/dev/null || true)" != "$project" ]; then
    fail "$binds exists, but this checkout did not record creating it for project $project. Nothing was removed"
fi
printf '%s\n' "$project" >"$record"
started=1
if [ -e "$binds" ]; then
    say "removing bind directories that an earlier run in this checkout left behind"
    remove_binds || fail "could not remove $binds"
fi

# --- Checks that need no build.
say 'checking that Compose refuses a missing password'
for value in no-password empty-password; do
    settings "$value"
    refusal=$(compose up -d --no-build 2>&1) && refused=0 || refused=1
    [ "$refused" = 1 ] || fail "Compose started with $value"
    case "$refusal" in *POSTGRES_PASSWORD*) ;; *) fail "the refusal does not name POSTGRES_PASSWORD: $refusal" ;; esac
    [ -z "$(owned)" ] || fail "Compose created something with $value"
done

say 'checking the PostgreSQL image pin'
settings volumes
compose config --images | grep -Fxq -- "$pin" \
    || fail "the db image in compose.yml differs from the pin in server/postgres-image.env ($pin)"

# mounts <service>: "type source target" for each mount in the resolved,
# normalized configuration, read with awk so the run needs no JSON tool.
mounts() {
    compose config 2>/dev/null | awk -v service="  $1:" '
        /^[^ ]/ { section = $0 }
        section == "services:" && /^  [^ ]/ { inside = ($0 == service) }
        inside && /^    [^ ]/ { volumes = ($0 == "    volumes:") }
        inside && volumes && /^      - type: / { if (type != "") print type, source, target; type = $3; source = ""; target = "" }
        inside && volumes && /^        source: / { source = $2 }
        inside && volumes && /^        target: / { target = $2 }
        END { if (type != "") print type, source, target }'
}

# expect_mounts <app mount> <db mount>: check both services' resolved mounts.
expect_mounts() {
    [ "$(mounts app)" = "$1" ] || fail "the app mounts $(mounts app), not $1"
    [ "$(mounts db)" = "$2" ] || fail "the db mounts $(mounts db), not $2"
}

say 'checking each mount selection'
named_storage="volume storage $storage"
named_database='volume database /var/lib/postgresql'
settings volumes
expect_mounts "$named_storage" "$named_database"
compose config | grep -Fxq "      STORAGE_ROOT: $storage" || fail "Compose does not set STORAGE_ROOT to $storage"
for files in compose.yml:compose.storage-bind.yml compose.yml:compose.database-bind.yml; do
    settings binds /srv/sidereal-verify-storage /srv/sidereal-verify-database
    sed -i.bak "s|^COMPOSE_FILE=.*|COMPOSE_FILE=$files|" "$envfile" && rm -f "$envfile.bak"
    case "$files" in
        *storage-bind*) expect_mounts "bind /srv/sidereal-verify-storage $storage" "$named_database" ;;
        *) expect_mounts "$named_storage" 'bind /srv/sidereal-verify-database /var/lib/postgresql' ;;
    esac
done
# Relative paths resolve against the project directory, never the app.
settings binds relative/storage relative/database
expect_mounts "bind $root/relative/storage $storage" "bind $root/relative/database /var/lib/postgresql"
for value in storage database; do
    settings binds
    sed -i.bak "s|^SIDEREAL_$(echo "$value" | tr a-z A-Z)_PATH=.*|SIDEREAL_$(echo "$value" | tr a-z A-Z)_PATH=|" "$envfile" && rm -f "$envfile.bak"
    refusal=$(compose config 2>&1) && fail "Compose accepted an empty $value path"
    case "$refusal" in *SIDEREAL_*_PATH*) ;; *) fail "the refusal does not name the empty path: $refusal" ;; esac
done
settings binds
compose config | grep -q 'create_host_path: false' || fail 'a bind override may create its host path'
[ "$(compose config | grep -c 'create_host_path: false')" = 2 ] || fail 'both bind overrides must refuse to create host paths'
for option in '      - ALL' '      - no-new-privileges:true'; do
    compose config | grep -Fxq -- "$option" || fail "the bind overrides lost the app option '${option#*- }'"
done

# --- Build, then start. The script measures the deadline itself: Compose's
# own wait timeout does not cover a dependency on every release.
say "building the app image for project $project"
settings volumes
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

sql() {
    compose exec -T db psql -U sidereal -d sidereal -v ON_ERROR_STOP=1 -Atc "$1"
}

migrations() {
    sql "SELECT version, encode(checksum, 'hex'), installed_on FROM _sqlx_migrations ORDER BY version"
}

# no_secret <text>...: fail when the logs of either service hold one of them.
no_secret() {
    logs=$(compose logs --no-color 2>&1)
    for secret in "$@"; do
        case "$logs" in *"$secret"*) fail 'the container logs hold a database password or connection URL' ;; esac
    done
}

# mounted <service>: "type source target" for each mount of the running container.
mounted() {
    docker inspect --format '{{range .Mounts}}{{.Type}} {{if eq .Type "volume"}}{{.Name}}{{else}}{{.Source}}{{end}} {{.Destination}}{{"\n"}}{{end}}' \
        "$(compose ps -q "$1")" | sed '/^$/d'
}

# retained: every value that must survive new containers.
retained() {
    cat "$envfile"
    echo '--- migrations'
    migrations
    echo '--- pairing'
    sql 'SELECT storage_id FROM sidereal_metadata'
    echo '--- storage'
    compose exec -T app sh -c 'cat "$STORAGE_ROOT/'"$marker"'" && echo --- && cat "$STORAGE_ROOT/sentinel"'
}

# instance <label> <app mount> <db mount>: check a first start, then
# recreate the containers and compare everything that must be retained.
instance() {
    start "$1, first start"
    # The mount check runs first, before any `down`: when it fails, the exit
    # trap's `down --volumes` still removes an unnamed volume.
    say "checking the mounts ($1)"
    [ "$(mounted app)" = "$2" ] || fail "the app mounts $(mounted app), not $2"
    [ "$(mounted db)" = "$3" ] || fail "the db mounts $(mounted db), not $3"

    say "checking the home page and the health probes ($1)"
    page=$(fetch /) || fail 'the home page did not load'
    [ "$page" = "$(compose exec -T app sh -c 'cat "$WEB_ROOT/index.html"')" ] || fail 'the home page is not index.html'
    curl -fsSI --max-time 5 "http://127.0.0.1:$port/" | grep -iq '^content-type: text/html' \
        || fail 'the home page is not text/html'
    [ "$(fetch /healthz)" = '{"status":"ok"}' ] || fail '/healthz did not return {"status":"ok"}'
    [ "$(fetch /readyz)" = '{"status":"ready"}' ] || fail '/readyz did not return {"status":"ready"}'

    say "checking the app user, its restrictions, and the database port ($1)"
    app=$(compose ps -q app)
    db=$(compose ps -q db)
    [ "$(compose exec -T app id -u):$(compose exec -T app id -g)" = 10001:10001 ] \
        || fail 'the app does not run as user and group 10001'
    [ "$(docker inspect --format '{{json .HostConfig.CapDrop}} {{json .HostConfig.SecurityOpt}}' "$app")" \
        = '["ALL"] ["no-new-privileges:true"]' ] || fail 'the app container lost its capability restrictions'
    [ "$(docker inspect --format '{{.RestartCount}}' "$app")" = 0 ] || fail 'the app restarted during its first start'
    [ "$(docker inspect --format '{{json .HostConfig.PortBindings}}' "$db")" = '{}' ] \
        || fail 'the db container publishes a host port'

    say "checking storage initialization ($1)"
    [ "$(sql 'SELECT count(*) FROM _sqlx_migrations WHERE success')" = 2 ] \
        || fail 'the database does not hold two successful migrations'
    [ "$(compose exec -T app ls -A "$storage")" = "$marker" ] \
        || fail "storage holds more than its identity marker after the first start"
    id=$(sql 'SELECT storage_id FROM sidereal_metadata')
    [ -n "$id" ] || fail 'the database is not paired with storage'
    compose exec -T app cat "$storage/$marker" | grep -Fq "\"storage_id\":\"$id\"" \
        || fail 'the storage marker does not carry the database pairing'
    compose exec -T app sh -c "touch $storage/probe && rm $storage/probe" \
        || fail 'the app user cannot write to storage'
    compose exec -T app sh -c "printf 'sentinel %s\n' $password | sha256sum >$storage/sentinel" \
        || fail 'could not write the storage sentinel'

    say "checking the logs after a successful start ($1)"
    no_secret "$password" 'postgres://'

    say "checking that the database and storage survive new containers ($1)"
    before=$(retained)
    compose down --timeout 10 >"$work/down.log" 2>&1 || fail "docker compose down failed; see $work/down.log"
    case "$3" in volume*) docker volume inspect "${project}_database" >/dev/null 2>&1 \
        || fail 'docker compose down removed the database volume' ;; esac
    start "$1, second start"
    [ "$(retained)" = "$before" ] || fail "settings, migrations, pairing, marker, or sentinel changed across docker compose down ($1)"
    [ "$(fetch /readyz)" = '{"status":"ready"}' ] || fail "/readyz did not return {\"status\":\"ready\"} after the second start ($1)"
}

# --- Absent bind sources: Compose refuses, creates no directory, and mounts
# no fallback volume.
say 'checking that absent bind directories are refused'
for service in app db; do
    settings binds "$binds/absent-storage" "$binds/absent-database"
    refusal=$(compose up -d --no-build --no-deps "$service" 2>&1) && fail "Compose started $service without its bind directory"
    case "$refusal" in *'does not exist'*) ;; *) fail "the refusal does not name the absent directory: $refusal" ;; esac
    [ ! -e "$binds/absent-storage" ] && [ ! -e "$binds/absent-database" ] || fail 'Compose created an absent bind directory'
    [ -z "$(docker ps -a -q --filter "label=com.docker.compose.project=$project")" ] \
        || fail "Compose created a $service container without its bind directory"
done
compose down --volumes --timeout 10 >"$work/down.log" 2>&1 || fail "docker compose down failed; see $work/down.log"

# --- Default named volumes.
settings volumes
instance 'named volumes' "volume ${project}_storage $storage" "volume ${project}_database /var/lib/postgresql"

# --- The app stays alive through a real database outage and recovers in place.
say 'checking database outage and recovery'
app=$(compose ps -q app)
restarts=$(docker inspect --format '{{.RestartCount}}' "$app")
outage_before=$(retained)
records_before=$(sql "SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY version), '[]')::text FROM _sqlx_migrations m WHERE success")
wait_health() {
    expected=$1
    waited=0
    until [ "$(docker inspect --format '{{.State.Health.Status}}' "$app")" = "$expected" ]; do
        if [ "$waited" -ge "$deadline" ]; then
            show_state
            fail "the app did not become $expected within $deadline seconds"
        fi
        sleep 1
        waited=$((waited + 1))
    done
}
compose stop db >"$work/outage.log" 2>&1 || fail 'could not stop the database'
wait_health unhealthy
[ "$(fetch /healthz)" = '{"status":"ok"}' ] || fail 'liveness failed during the database outage'
[ "$(curl -sS --max-time 3 "http://127.0.0.1:$port/readyz")" = '{"status":"not_ready"}' ] \
    || fail 'readiness did not report the database outage'
compose logs --no-color --no-log-prefix app | grep -Fxq 'readiness: not ready (database unreachable)' \
    || fail 'the app did not log the database outage'
compose start db >>"$work/outage.log" 2>&1 || fail 'could not start the database'
wait_health healthy
[ "$(fetch /readyz)" = '{"status":"ready"}' ] || fail 'readiness did not recover'
compose logs --no-color --no-log-prefix app | grep -Fxq 'readiness: ready' \
    || fail 'the app did not log recovery'
[ "$(docker inspect --format '{{.RestartCount}}' "$app")" = "$restarts" ] \
    || fail 'the app restarted during the database outage'
[ "$(retained)" = "$outage_before" ] || fail 'the outage changed settings, migration records, pairing, marker, or sentinel'
[ "$(sql "SELECT coalesce(jsonb_agg(to_jsonb(m) ORDER BY version), '[]')::text FROM _sqlx_migrations m WHERE success")" = "$records_before" ] \
    || fail 'the outage changed successful migration records'
no_secret "$password" 'postgres://' 'postgresql://'

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
compose down --volumes --timeout 10 >"$work/down.log" 2>&1 || fail "docker compose down failed; see $work/down.log"

# --- Both bind overrides, retained through COMPOSE_FILE in the env file. The
# storage directory is prepared for the app user without host root.
say 'preparing this run'"'"'s bind directories'
mkdir -p "$binds/storage" "$binds/database" "$binds/empty-storage"
helper "$binds/storage" chown 10001:10001 /target || fail 'could not prepare the storage bind directory'
helper "$binds/empty-storage" chown 10001:10001 /target || fail 'could not prepare the empty storage directory'
settings binds
instance 'bind mounts' "bind $binds/storage $storage" "bind $binds/database /var/lib/postgresql"

# --- A paired database with an empty directory at its storage target must
# refuse to serve, create nothing, and keep its pairing.
say 'checking that a paired database refuses empty storage'
id=$(sql 'SELECT storage_id FROM sidereal_metadata')
settings binds "$binds/empty-storage"
compose up -d --no-build --no-deps --force-recreate app >"$work/up.log" 2>&1 \
    || fail "could not start the app against empty storage; see $work/up.log"
app=$(compose ps -a -q app)
tries=0
until [ "$(docker inspect --format '{{if or (gt .RestartCount 0) (and (eq .State.Status "exited") (ne .State.ExitCode 0))}}failed{{end}}' "$app")" = failed ]; do
    tries=$((tries + 1))
    if [ "$tries" -ge 90 ]; then
        show_state
        fail 'the app did not refuse empty storage'
    fi
    sleep 1
done
logs=$(compose logs --no-color app 2>&1)
case "$logs" in *'storage marker is missing'*) ;; *) fail "the refusal does not name the missing marker: $logs" ;; esac
case "$logs" in *'listening on'*) fail 'the app started an HTTP listener against empty storage' ;; esac
curl -fsS --max-time 5 "http://127.0.0.1:$port/healthz" >/dev/null 2>&1 && fail 'the app served HTTP against empty storage'
[ -z "$(ls -A "$binds/empty-storage")" ] || fail 'the refused start created a file in empty storage'
[ "$(sql 'SELECT storage_id FROM sidereal_metadata')" = "$id" ] || fail 'the refused start changed the database pairing'
no_secret "$password" 'postgres://'

say "passed. Removing project $project"
