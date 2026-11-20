#!/bin/sh
# Checks how `sh scripts/dev.sh` supervises its two workers, without the
# real server or web shell: a scratch copy of the script runs stub cargo
# and pnpm commands. Each case runs under dash and bash --posix.
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

mkdir -p "$fx/app/scripts" "$fx/app/web" "$fx/bin"
cp "$root/scripts/dev.sh" "$fx/app/scripts/dev.sh"
# Each stub records its PID, then runs the case's script for that worker.
cat >"$fx/bin/cargo" <<'EOF'
#!/bin/sh
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

failures=0
fail() {
  echo "FAIL [$label]: $*" >&2
  failures=$((failures + 1))
}

# start SHELL...: starts the supervisor in the background as $sup.
start() {
  : >"$fx/pids"
  (cd "$fx/app" && PATH="$fx/bin:$PATH" exec perl -e '$SIG{INT} = "DEFAULT"; exec @ARGV or die' "$@" scripts/dev.sh) \
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

# Both workers stop well inside one poll of the supervisor.
exit_status() { # NAME SERVER WEB EXPECTED
  label="$shell_name: $1"
  STUB_SERVER=$2 STUB_WEB=$3 FX=$fx
  export STUB_SERVER STUB_WEB FX
  # shellcheck disable=SC2086
  start $shell
  finish
  [ "$status" -eq "$4" ] || fail "exit status $status, expected $4"
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
  exit_status "the web shell fails, then the server stops cleanly" \
    'sleep 0.4; exit 0' 'sleep 0.1; exit 23' 23
  exit_status "the server stops cleanly, then the web shell fails" \
    'sleep 0.1; exit 0' 'sleep 0.4; exit 23' 23
  exit_status "the web shell fails, then the server fails" \
    'sleep 0.4; exit 7' 'sleep 0.1; exit 23' 23
  exit_status "the server fails first" \
    'sleep 0.1; exit 7' 'exec sleep 30' 7
done

if [ "$failures" -gt 0 ]; then
  echo "dev-supervisor-check: $failures failed" >&2
  exit 1
fi
echo "dev-supervisor-check: all cases passed"
