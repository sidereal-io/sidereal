# Sidereal v2 backend

The Rust backend for Sidereal v2, a cargo workspace living as a sibling subtree
alongside the existing TypeScript stack (`apps/`, `packages/`). See
[`docs/architecture.md`](../docs/architecture.md) and
[ADR-002](../docs/decisions/ADR-002-core-domain-pack-split.md) for the design.

## Crate layout

```
server/
  Cargo.toml            # workspace manifest (members + shared dep versions)
  rust-toolchain.toml   # names the stable channel
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

- **[rustup](https://rustup.rs/)** — installs cargo and the latest stable Rust, which
  `rust-toolchain.toml` selects. Run `rustup update` to move to a newer release. A C
  linker is also required (`build-essential` on Debian/Ubuntu, Xcode CLT on macOS).
- **[just](https://github.com/casey/just)** — the command runner spanning both stacks:
  `cargo install just` (or a system package: `apt install just`, `brew install just`,
  `scoop install just`).

The repo also provides an optional, pinned Nix shell with every prerequisite above.
See [`CONTRIBUTING.md`](../CONTRIBUTING.md#development-environment).

## Zero-to-running

From the **repository root**:

```bash
just server
```

That builds and starts the Rust backend. It serves its liveness probe once up:

```bash
curl localhost:5000/healthz     # -> 200 {"status":"ok"}
```

## Recipes

`just` recipes live in the root `justfile`. These are the backend's; `just --list`
shows every recipe in the repo.

| Recipe | What it does |
|---|---|
| `just server` | Rust backend only. |
| `just check` | The gate to pass before every PR. It runs `just check-server`. |
| `just check-server` | `cargo fmt --check` + `clippy -D warnings` + `cargo test` + arch lint. CI runs this recipe. |

A Rust-only contributor can skip `just` and call cargo directly from `server/`
(the pinned toolchain is auto-selected there):

```bash
cd server
cargo run -p sidereal-server    # boot the server
cargo test                      # run the workspace tests
```

## Configuration

| Variable | Default | Description |
|---|---|---|
| `PORT` | `5000` | Server listen port. |

## Notes

- The `justfile` lives at the repo root (not here) because it runs every part of
  the repo, not only the backend.
- `plugin-abi` is intentionally minimal in M0 — trait stubs, not a frozen ABI;
  it is expected to churn until M2.
