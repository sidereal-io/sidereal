# Sidereal v2 backend

The Rust backend for Sidereal v2, a cargo workspace living as a sibling subtree
alongside the existing TypeScript stack (`apps/`, `packages/`). See
[`docs/architecture/README.md`](../docs/architecture/README.md) and
[ADR-002](../docs/decisions/ADR-002-core-domain-pack-split.md) for the design.

## Crate layout

```
backend/
  Cargo.toml            # workspace manifest (members + shared dep versions)
  rust-toolchain.toml   # pins stable 1.85
  crates/
    plugin-abi/         # public plugin contracts a third-party pack also codes against
    core/               # domain-agnostic engine; builds on plugin-abi (no astro)
    server/             # thin axum binary; serves GET /healthz, wires core + packs
    packs/
      astro/            # first-party pack; depends on plugin-abi ONLY, never core
  scripts/
    check-arch.sh       # dependency-direction lint: forbids packs/astro -> core
```

## Prerequisites

- **[rustup](https://rustup.rs/)** — installs cargo and, on first build, auto-selects
  the toolchain pinned in `rust-toolchain.toml` (stable 1.85). A C linker is also
  required (`build-essential` on Debian/Ubuntu, Xcode CLT on macOS).
- **[just](https://github.com/casey/just)** — the command runner spanning both stacks:
  `cargo install just` (or a system package: `apt install just`, `brew install just`,
  `scoop install just`).
- **[Node.js](https://nodejs.org/) 20+** — only needed to run the frontend half of
  `just dev`.

## Zero-to-running

From the **repository root**:

```bash
just dev
```

That starts the Rust backend and the Vite frontend together. The backend serves
its liveness probe once up:

```bash
curl localhost:5000/healthz     # -> 200 {"status":"ok"}
```

## Recipes

`just` recipes live in the root `justfile`; `just --list` self-documents them.

| Recipe | What it does |
|---|---|
| `just dev` | Backend + frontend together (zero-to-running). |
| `just backend` | Rust backend only. |
| `just frontend` | Vite frontend only. |
| `just check` | `cargo fmt --check` + `clippy -D warnings` + `cargo test` + arch lint. |

A Rust-only contributor can skip `just` and call cargo directly from `backend/`
(the pinned toolchain is auto-selected there):

```bash
cd backend
cargo run -p sidereal-server    # boot the server
cargo test                      # run the workspace tests
```

## Configuration

| Variable | Default | Description |
|---|---|---|
| `PORT` | `5000` | Server listen port. |

## Notes

- The `justfile` lives at the repo root (not here) because `just dev` spans both
  the Rust backend and the TypeScript frontend.
- `plugin-abi` is intentionally minimal in M0 — trait stubs, not a frozen ABI;
  it is expected to churn until M2.
