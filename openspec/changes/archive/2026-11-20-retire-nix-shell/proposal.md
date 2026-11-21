<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Sidereal now has two routes to its pinned tools: the Nix shell and mise. Every job and task already runs on mise, and the weekly workflow keeps the mise pins current. Keeping the Nix route means two places to change each shared pin, a Let's Spec tag in `flake.nix` that nothing moves, and docs that tell Nix users to run mise on the host anyway. ADR-016 decided to retire the flake, and this change, the last story of epic #496, removes it.

## What Changes

- **BREAKING** for contributors who use the Nix shell: the flake, `nix/`, `.envrc`, the `nix` workflow and Dependabot's `nix` entry are deleted (D1). Entering the repo with direnv no longer loads a shell. `CONTRIBUTING.md` gets a short note on moving to mise (D14).
- `.gitignore` keeps `.direnv/` and `.envrc.local`, under a comment saying they are leftovers from the retired Nix route (D5).
- The `dev-environment` spec loses every requirement that holds only on the Nix route or through direnv (D6, D7). Requirements that name both routes keep their mise and hand-install parts (D8).
- The `ci` spec stops naming the Nix files as triggers for the `server` job (D9). The `dev-commands` spec stops naming Nix in its no-mise requirement (D10).
- `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `server/README.md`, `web/README.md` and `docs/dependency-updates.md` describe two routes only: mise, and tools installed by hand (D11). `CONTRIBUTING.md` lists OpenSSL as a system prerequisite for `db-tls` and `external-db-verify` (D13). The NixOS note on Playwright stays (D15).
- Comments in `mise.toml` and `scripts/check-mise-lock-platforms.sh` stop pointing at the Nix shell and `flake.nix` (D16).
- No new decision record: ADR-016 already decides this (D3).
- Out of scope: a Nix route beside mise, the `v0.x` branch, and the history in archived changes and ADR-013 (D2). `openspec/discovery.md` waits for the next `lets-shape` pass (D17).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: removes the Nix shell, flake check, direnv and Nix Dependabot requirements. Keeps the plain pin files and the hand-install route. Drops the Nix wording from the Rust, Node, Let's Spec, gate and OpenSpec-warning requirements.
- `ci`: the `server` job's trigger list loses `flake.nix`, `flake.lock` and `nix/`, and the scenario for a pull request that changes only the shell's pins goes.
- `dev-commands`: `CONTRIBUTING.md` lists each task's command for contributors without mise, with no mention of Nix.

## Impact

- **Deleted files:** `flake.nix`, `flake.lock`, `nix/devshell.nix`, `nix/lets-spec.nix`, `nix/openspec.nix`, `nix/toolchains.nix`, `.envrc`, `.github/workflows/nix.yml`.
- **Changed configuration:** `.github/dependabot.yml` (the `nix` entry goes), `.gitignore` (comment only), `mise.toml` and `scripts/check-mise-lock-platforms.sh` (comments only). No tool version moves.
- **Changed docs:** `AGENTS.md` (the "Working in this repo" section and the durable constraints only, never the lets-spec block), `CONTRIBUTING.md`, `README.md`, `server/README.md`, `web/README.md`, `docs/dependency-updates.md`.
- **Specs:** `openspec/specs/dev-environment/spec.md`, `openspec/specs/ci/spec.md`, `openspec/specs/dev-commands/spec.md`.
- **CI:** the `flake check` job stops running. No ruleset requires it, so merges to `main` are not blocked (D4).
- **Contributors on the Nix shell:** they install mise, from Nixpkgs if they like, and follow "With mise" in `CONTRIBUTING.md`.
- **Acceptance:** see D18.
