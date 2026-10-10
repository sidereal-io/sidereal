# Sidereal — the one place to run the server (server/) and the web shell
# (web/) for development. The v0.10.x app lives on the v0.x branch.
# See https://github.com/casey/just, server/README.md, and web/README.md.

# Show available recipes.
default:
    @just --list

# Start this worktree's loopback PostgreSQL fixture and wait for initialization.
db-up:
    @sh server/scripts/db-fixture.sh up

# Stop this worktree's database fixture, retaining its volume.
db-down:
    @sh server/scripts/db-fixture.sh down

# Delete only this worktree's fixture containers and database volume.
db-clean:
    @sh server/scripts/db-fixture.sh clean

# Print the fixture runtime URL. Override SIDEREAL_DB_PORT if needed.
db-url:
    @sh server/scripts/db-fixture.sh url

# Print the separate CREATEDB fixture test URL.
db-test-url:
    @sh server/scripts/db-fixture.sh test-url

# Generate the OpenSpec agent skills into .agents/skills (see scripts/skills.sh).
skills:
    @scripts/skills.sh

# Refresh stale OpenSpec skills. The dev shell runs this when it loads.
enter:
    @scripts/skills.sh --if-stale >/dev/null 2>&1 || echo "warning: could not refresh the OpenSpec skills; run \`just skills\` to see why" >&2

# Set each issue's Status on the project board from its state (see scripts/board-sync.sh).
# Pass --dry-run to print the changes without writing them.
board-sync *args:
    @scripts/board-sync.sh {{args}}

# Check the board's Status rules against a fixture (see scripts/board-status-test.sh).
board-sync-test:
    @scripts/board-status-test.sh

# When either one exits, stop the other. POSIX sh, so macOS's /bin/sh runs it.
# Run both recipes in parallel; press Ctrl+C to stop them.
[doc('Run the server and web shell together.')]
[parallel]
dev: server web

# Run the server against DATABASE_URL (GET /healthz and /readyz).
server:
    cargo run -p sidereal-server --manifest-path server/Cargo.toml

# Run the web shell only. Installs from web/pnpm-lock.yaml first.
web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm dev

# The gate to pass before every PR: runs check-server, then check-web.
check: check-server check-web

# Server gate: format, lint (deny warnings), tests, dependency-direction lint.
check-server:
    cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    server/scripts/check-arch.sh

# Web gate: install from the lockfile, then type check, lint, format, DESIGN.md lint, token drift, tests.
check-web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck && pnpm lint && pnpm format:check && pnpm design:lint && pnpm tokens:check && pnpm test

# Remove killed-test leftovers only from this worktree's fixture.
db-test-clean:
    @sh server/scripts/db-fixture.sh orphan-clean

# Regenerate normalized schema descriptions using the pinned fixture.
db-schema:
    #!/bin/sh
    set -eu
    just db-up
    export TEST_DATABASE_URL=$(just db-test-url)
    SIDEREAL_GENERATE_SCHEMA=1 cargo test --manifest-path server/Cargo.toml -p sidereal-core schema_baselines_are_generated_and_stable

# Generate local fixture certificates without requiring Docker.
db-tls:
    @sh server/scripts/db-tls.sh
