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

# The gate to pass before every PR: Compose configuration, server, then web.
check: check-compose check-server check-web

# Check Compose installation wiring; needs the Compose CLI and jq, no daemon.
check-compose:
    @sh scripts/compose-config-check.sh

# Server gate: format, lint (deny warnings), tests, dependency-direction lint.
check-server:
    cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    server/scripts/check-arch.sh

# Web gate: install from the lockfile, then type check, lint, format, DESIGN.md lint, token drift, tests.
check-web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck && pnpm lint && pnpm format:check && pnpm design:lint && pnpm tokens:check && pnpm test

# Needs Docker, and takes minutes, so `check` does not run it.
# Build the app image, start a disposable Compose instance, check it, and remove it.
instance-verify:
    @sh scripts/instance-verify.sh

# Check TLS startup and recreation against a separate disposable PostgreSQL service.
external-db-verify:
    @sh scripts/external-db-verify.sh

# End-to-end tests: boot the server and Vite, then drive Chromium against them.
# The database fixture must already be up (`just db-up`); the webServer blocks
# in web/playwright.config.ts start everything else. Reuses an already-running
# `just dev`, so in a normal dev shell this only pays for the browser launch.
[doc('Run the Playwright end-to-end suite.')]
e2e:
    #!/bin/sh
    set -eu
    # The server reads DATABASE_URL, not TEST_DATABASE_URL, and exits without
    # it. Fall back to the local fixture when the caller has not picked one;
    # `db-schema` resolves its own URL the same way.
    if [ -z "${DATABASE_URL:-}" ]; then
      DATABASE_URL=$(just db-url)
      export DATABASE_URL
    fi
    cd web
    pnpm install --frozen-lockfile --reporter=append-only
    # Playwright owns its own browser. The download is skipped when the
    # browser this version wants is already in the cache, so a bare
    # `just e2e` keeps working with no setup step.
    pnpm exec playwright install chromium
    pnpm e2e

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
