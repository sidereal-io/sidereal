#!/bin/sh
# Checks how `sh scripts/dev.sh` supervises its two workers, without the
# real server or web shell: a scratch copy of the script runs stub cargo
# and pnpm commands. Each case runs under dash and bash --posix, once
# with setsid on PATH and once with only the perl fallback.
#
#   sh scripts/dev-supervisor-check.sh
#
# Needs perl: it starts the supervisor with SIGINT restored, because a
# background job of a non-interactive shell starts with SIGINT ignored.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
command -v perl >/dev/null 2>&1 || { echo "dev-supervisor-check: perl is required" >&2; exit 1; }

fx=$(mktemp -d "${TMPDIR:-/tmp}/dev-supervisor-check.XXXXXX")
cleanup() {
  # A failing case can leave stub processes behind; stop them.
  if [ -f "$fx/pids" ]; then
    # shellcheck disable=SC2046
    kill -KILL $(cat "$fx/pids") 2>/dev/null || :
  fi
  rm -rf "$fx"
}
trap cleanup EXIT
trap 'exit 130' HUP INT TERM

mkdir -p "$fx/app/scripts" "$fx/app/web" "$fx/bin" "$fx/nosetsid"
cp "$root/scripts/dev.sh" "$fx/app/scripts/dev.sh"
# Each stub records its PID, then runs the case's script for that worker.
# The cargo stub also records each call; a build call exits with
# $STUB_BUILD, 0 by default, and starts nothing.
cat >"$fx/bin/cargo" <<'EOF'
#!/bin/sh
echo "$*" >>"$FX/cargo-calls"
[ "$1" = build ] && exit "${STUB_BUILD:-0}"
echo $$ >>"$FX/pids"
eval "$STUB_SERVER"
EOF
cat >"$fx/bin/pnpm" <<'EOF'
#!/bin/sh
[ "$1" = install ] && exit 0
echo $$ >>"$FX/pids"
eval "$STUB_WEB"
EOF
chmod +x "$fx/bin/cargo" "$fx/bin/pnpm"
# The perl route: a PATH with only the commands the script and stubs need.
for tool in sh dirname mktemp rm sleep perl cat; do
  ln -s "$(command -v "$tool")" "$fx/nosetsid/$tool"
done
ln -s "$fx/bin/cargo" "$fx/bin/pnpm" "$fx/nosetsid/"

failures=0
fail() {
  echo "FAIL [$label]: $*" >&2
  failures=$((failures + 1))
}

# start SHELL...: starts the supervisor in the background as $sup.
start() {
  : >"$fx/pids"
  : >"$fx/cargo-calls"
  rm -f "$fx/child"
  (cd "$fx/app" && PATH=$route_path exec perl -e '$SIG{INT} = "DEFAULT"; exec @ARGV or die' "$@" scripts/dev.sh) \
    >"$fx/log" 2>&1 &
  sup=$!
}

# finish: waits up to 15 seconds for the supervisor; sets $status.
finish() {
  tries=0
  while kill -0 "$sup" 2>/dev/null && [ "$tries" -lt 150 ]; do
    sleep 0.1
    tries=$((tries + 1))
  done
  if kill -0 "$sup" 2>/dev/null; then
    fail "the supervisor was still running after 15 seconds"
    kill -KILL "$sup" 2>/dev/null || :
  fi
  status=0
  wait "$sup" || status=$?
}

# gone PID: true once PID has stopped, allowing a second for reaping.
gone() {
  tries=0
  while kill -0 "$1" 2>/dev/null; do
    [ "$tries" -lt 10 ] || return 1
    sleep 0.1
    tries=$((tries + 1))
  done
}

# Both workers stop well inside one poll of the supervisor.
exit_status() { # NAME SERVER WEB EXPECTED
  label="$shell_name, $route: $1"
  STUB_SERVER=$2 STUB_WEB=$3 FX=$fx
  export STUB_SERVER STUB_WEB FX
  # shellcheck disable=SC2086
  start $shell
  finish
  [ "$status" -eq "$4" ] || fail "exit status $status, expected $4"
}

finished_worker_child() {
  label="$shell_name, $route: a finished worker's child stops"
  STUB_SERVER='exec sleep 30'
  STUB_WEB='sleep 30 & echo $! >"$FX/child"; echo $! >>"$FX/pids"; exit 23'
  FX=$fx
  export STUB_SERVER STUB_WEB FX
  # shellcheck disable=SC2086
  start $shell
  finish
  [ "$status" -eq 23 ] || fail "exit status $status, expected 23"
  [ -s "$fx/child" ] || { fail "the web stub started no child"; return; }
  gone "$(cat "$fx/child")" || fail "the web worker's child outlived the supervisor"
}

interrupt_detached_descendant() {
  label="$shell_name, $route: Ctrl-C stops a detached descendant"
  STUB_SERVER='exec sleep 30'
  # The subshell exits at once, so its sleep is reparented away from the worker.
  STUB_WEB='(sleep 30 & echo $! >"$FX/child"; echo $! >>"$FX/pids"); exec sleep 30'
  FX=$fx
  export STUB_SERVER STUB_WEB FX
  # shellcheck disable=SC2086
  start $shell
  tries=0
  until [ -s "$fx/child" ] || [ "$tries" -ge 50 ]; do sleep 0.1; tries=$((tries + 1)); done
  sleep 0.3
  kill -INT "$sup"
  finish
  [ "$status" -eq 130 ] || fail "exit status $status, expected 130"
  [ -s "$fx/child" ] || { fail "the web stub started no child"; return; }
  gone "$(cat "$fx/child")" || fail "the detached descendant outlived the supervisor"
}

server_builds_the_helper_first() {
  label="$shell_name, $route: the server worker builds both binaries, then runs"
  STUB_SERVER='exit 0' STUB_WEB='exec sleep 30' FX=$fx
  export STUB_SERVER STUB_WEB FX
  # shellcheck disable=SC2086
  start $shell
  finish
  [ "$status" -eq 0 ] || fail "exit status $status, expected 0"
  build=$(sed -n 1p "$fx/cargo-calls")
  run=$(sed -n 2p "$fx/cargo-calls")
  case "$build" in
    build*--bin\ sidereal-server*--bin\ sidereal-preview-worker*) ;;
    *) fail "the first cargo call was '$build', not a build of both binaries" ;;
  esac
  case "$run" in
    run\ -p\ sidereal-server*) ;;
    *) fail "the second cargo call was '$run', not the server's run" ;;
  esac
}

failed_build_runs_no_server() {
  label="$shell_name, $route: a failed build starts no server"
  STUB_SERVER='exit 0' STUB_WEB='exec sleep 30' STUB_BUILD=9 FX=$fx
  export STUB_SERVER STUB_WEB STUB_BUILD FX
  # shellcheck disable=SC2086
  start $shell
  finish
  unset STUB_BUILD
  [ "$status" -eq 9 ] || fail "exit status $status, expected 9"
  [ "$(wc -l <"$fx/cargo-calls" | tr -d ' ')" -eq 1 ] || fail "cargo ran after the failed build"
}

shells=
command -v dash >/dev/null 2>&1 && shells="dash"
command -v bash >/dev/null 2>&1 && shells="$shells bash"
[ -n "$shells" ] || { echo "dev-supervisor-check: needs dash or bash" >&2; exit 1; }
[ -z "${shells##*dash*}" ] || echo "dev-supervisor-check: no dash here; running bash --posix only" >&2

for shell_name in $shells; do
  case "$shell_name" in
    dash) shell=$(command -v dash) ;;
    bash) shell="$(command -v bash) --posix" ;;
  esac
  for route in setsid perl; do
    if [ "$route" = setsid ]; then
      command -v setsid >/dev/null 2>&1 || continue
      route_path="$fx/bin:$PATH"
    else
      route_path="$fx/nosetsid"
    fi
    exit_status "the web shell fails, then the server stops cleanly" \
      'sleep 0.4; exit 0' 'sleep 0.1; exit 23' 23
    exit_status "the server stops cleanly, then the web shell fails" \
      'sleep 0.1; exit 0' 'sleep 0.4; exit 23' 23
    exit_status "the web shell fails, then the server fails" \
      'sleep 0.4; exit 7' 'sleep 0.1; exit 23' 23
    exit_status "the server fails first" \
      'sleep 0.1; exit 7' 'exec sleep 30' 7
    finished_worker_child
    interrupt_detached_descendant
    server_builds_the_helper_first
    failed_build_runs_no_server
  done
done

if [ "$failures" -gt 0 ]; then
  echo "dev-supervisor-check: $failures failed" >&2
  exit 1
fi
echo "dev-supervisor-check: all cases passed"
