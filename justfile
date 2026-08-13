# Sidereal — single command front door spanning the Rust backend and the
# Vite frontend. See https://github.com/casey/just and backend/README.md.

# Show available recipes.
default:
    @just --list

# Zero-to-running: backend + frontend together.
dev:
    npx concurrently -n backend,frontend -c blue,green \
      "cargo run -p sidereal-server --manifest-path backend/Cargo.toml" \
      "npm run dev:frontend"

# Run the Rust backend only (serves GET /healthz).
backend:
    cargo run -p sidereal-server --manifest-path backend/Cargo.toml

# Run the Vite frontend only.
frontend:
    npm run dev:frontend

# Backend checks: format, lint (deny warnings), tests, dependency-direction lint.
check:
    cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    backend/scripts/check-arch.sh
