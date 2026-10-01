# Sidereal — single command front door spanning the Rust backend and the
# Vite frontend. See https://github.com/casey/just and server/README.md.

# Show available recipes.
default:
    @just --list

# Generate the OpenSpec agent skills into .agents/skills (see scripts/skills.sh).
skills:
    @scripts/skills.sh

# Refresh stale OpenSpec skills. The dev shell runs this when it loads.
enter:
    @scripts/skills.sh --if-stale >/dev/null 2>&1 || echo "warning: could not refresh the OpenSpec skills; run \`just skills\` to see why" >&2

# Run the v2 server and the v2 web shell together. When either one exits,
# stop the other. POSIX sh, so macOS's /bin/sh runs it too.
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

# Run the Rust server only (serves GET /healthz).
server:
    cargo run -p sidereal-server --manifest-path server/Cargo.toml

# Run the v2 web shell only. Installs from web/pnpm-lock.yaml first.
[group('v2')]
web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm dev

# Run the Vite frontend only.
frontend:
    npm run dev:frontend

# The gate to pass before every PR: runs check-server.
check: check-server

# Server gate: format, lint (deny warnings), tests, dependency-direction lint.
check-server:
    cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    server/scripts/check-arch.sh
