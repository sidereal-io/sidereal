# Sidereal — the one place to run each stack for development: the v2 server
# (server/), the v2 web shell (web/), and the v0.10.x stack (apps/, packages/).
# See https://github.com/casey/just, server/README.md, and web/README.md.

# Show available recipes.
default:
    @just --list

# Generate the OpenSpec agent skills into .agents/skills (see scripts/skills.sh).
skills:
    @scripts/skills.sh

# Refresh stale OpenSpec skills. The dev shell runs this when it loads.
enter:
    @scripts/skills.sh --if-stale >/dev/null 2>&1 || echo "warning: could not refresh the OpenSpec skills; run \`just skills\` to see why" >&2

# When either one exits, stop the other. POSIX sh, so macOS's /bin/sh runs it.
[doc('Run the v2 server and web together.')]
[group('v2')]
dev:
    #!/bin/sh
    just server & s=$!
    just web & w=$!
    trap 'stop=1; kill $s $w 2>/dev/null' INT TERM
    while kill -0 $s 2>/dev/null && kill -0 $w 2>/dev/null; do sleep 1; done
    kill $s $w 2>/dev/null
    wait $s; a=$?; wait $w; b=$?
    [ -n "$stop" ] && exit 0
    [ $a -eq 0 ] && [ $b -eq 0 ]

# Run the v2 server only (serves GET /healthz).
[group('v2')]
server:
    cargo run -p sidereal-server --manifest-path server/Cargo.toml

# Run the v2 web shell only. Installs from web/pnpm-lock.yaml first.
[group('v2')]
web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm dev

# Run the whole v0.10.x stack: its server, worker, and frontend.
[group('v0.10.x')]
v0-dev:
    npm run dev

# Run the v0.10.x frontend only.
[group('v0.10.x')]
v0-frontend:
    npm run dev:frontend

# The gate to pass before every PR: runs check-server.
[group('v2')]
check: check-server

# Server gate: format, lint (deny warnings), tests, dependency-direction lint.
[group('v2')]
check-server:
    cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    server/scripts/check-arch.sh
