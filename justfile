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

# Zero-to-running: backend + frontend together.
dev:
    npx concurrently -n backend,frontend -c blue,green \
      "cargo run -p sidereal-server --manifest-path server/Cargo.toml" \
      "npm run dev:frontend"

# Run the Rust server only (serves GET /healthz).
server:
    cargo run -p sidereal-server --manifest-path server/Cargo.toml

# Run the Vite frontend only.
frontend:
    npm run dev:frontend

# Backend checks: format, lint (deny warnings), tests, dependency-direction lint.
check:
    cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    server/scripts/check-arch.sh
