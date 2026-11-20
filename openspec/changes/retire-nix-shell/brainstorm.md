<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#502 retire-nix-shell — the Nix shell is gone, and the docs describe the mise and hand-install routes. https://github.com/sidereal-io/sidereal/issues/502

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which files does the change delete? | `flake.nix`, `flake.lock`, `nix/` (`devshell.nix`, `lets-spec.nix`, `openspec.nix`, `toolchains.nix`), `.envrc`, `.github/workflows/nix.yml`, and the `nix` entry in `.github/dependabot.yml`. | Evidence: #502 body (Scope, in) |
| D2 | What stays out of scope? | A Nix route beside mise, and the `v0.x` branch. Archived changes and ADR-013 keep their Nix text as history. | Evidence: #502 body (Scope, out); ADR-013 status `superseded by ADR-016` |
| D3 | Does retiring Nix need a new decision record? | No. ADR-016 is Accepted, supersedes ADR-013, and already decides that the flake, `nix/`, `.envrc` and the `nix` workflow go. | Evidence: `docs/decisions/ADR-016-development-environment-with-mise.md` |
| D4 | Does deleting the `nix` workflow block merges to `main`? | No. The `main required checks` ruleset requires only `server`, `web`, `codeql (rust)` and `codeql (javascript-typescript)`; the `flake check` is not required. | Evidence: GitHub ruleset 25483647 |
| D5 | Do `.direnv/` and `.envrc.local` leave `.gitignore`? | No. Both stay ignored, under a comment saying they are leftovers from the retired Nix route, so `git add -A` cannot commit a former Nix user's personal settings, which may hold secrets. | Assumption |
| D6 | Which `dev-environment` requirements go? | Every requirement that holds only on the Nix route or through direnv: the Nix shell's toolchain, the flake check and its CI job, the supported Nix systems, direnv entry and `.envrc.local`, Dependabot's Nix updates, the Nix shell's OpenSpec release and its flake-check guard. The plain pin files (`server/rust-toolchain.toml`, `.nvmrc`) and the hand-install scenario stay. | Evidence: #502 body (Scope, in) |
| D7 | Are the three Nix-only integrity requirements carried over to mise: every flake input locked and fetched scripts hash-checked; the same commit gives the same shell; the skills refresh works without a network? | No, all three are dropped. `mise.lock` already records a checksum for every download, and `OPENSPEC_NO_UPDATE_CHECK` stays set through mise. An offline `lets sync` under mise would be a new, untested promise outside this story. | Assumption |
| D8 | What happens to `dev-environment` requirements that name both routes or the Nix shell in passing? | They keep their mise and hand-install parts and lose the Nix parts. This covers "Both routes use one Let's Spec release", "Entering the repo warns while the skills lag OpenSpec 2", "The repo checks pass through mise", "Rust follows the latest stable release", "Contributors and CI name one Node version", and the spec's Purpose. | Evidence: #502 body (Scope, in); `openspec/specs/dev-environment/spec.md` |
| D9 | Does the `ci` spec change? | Yes, only its Nix parts: "The server checks run when server code changes" loses `flake.nix`, `flake.lock` and `nix/` as triggers and the scenario "A pull request changes only the shell's pins". The older mismatch (`ci.yml` has no path filter, so a docs-only pull request runs the server job) stays out of this change, and the pull request names it. | Assumption |
| D10 | Does the `dev-commands` spec change? | Yes. "The development tasks run without mise" says `CONTRIBUTING.md` lists commands "for contributors with neither mise nor Nix"; it becomes "for contributors without mise". | Evidence: #496 body (Must: the `dev-environment`, `dev-commands` and `ci` specs match) |
| D11 | Which documents change? | `AGENTS.md` (the "Working in this repo" section and the durable constraints, never the lets-spec block between its markers), `CONTRIBUTING.md` (including "Where versions are set", the Let's Spec update steps, and the interim text about Nix users running mise on the host), `README.md`, `server/README.md`, `web/README.md` and `docs/dependency-updates.md`. Each describes the mise route and the hand-install route only. | Evidence: #502 body (Scope, in); `git grep` for Nix, flake and direnv |
| D12 | Does `docs/security-scanning.md` change? | No. It names no Nix route, flake or direnv. | Evidence: `docs/security-scanning.md` |
| D13 | How does `CONTRIBUTING.md` list OpenSSL? | As a system prerequisite for the `db-tls` and `external-db-verify` tasks, on both routes, replacing the "Where versions are set" row that says `flake.lock` pins it. | Evidence: #502 body (Scope, in) |
| D14 | Does `CONTRIBUTING.md` help contributors who used the Nix shell? | Yes, with a short note: run `direnv revoke` in the repo, delete `.direnv/`, move `.envrc.local` settings to `mise.local.toml`, then follow "With mise". It adds that mise can be installed from Nixpkgs. | Assumption |
| D15 | Does `web/README.md` keep its note on running Playwright on NixOS? | Yes. The note is about NixOS hosts, which still exist without the Nix shell. Only its wording about the shell changes. | Evidence: `web/README.md` (Playwright section) |
| D16 | Do code comments that point at the Nix shell change? | Yes. The comment in `mise.toml` that compares with the Nix shell, and the comment in `scripts/check-mise-lock-platforms.sh` that takes the supported systems from `flake.nix`, name mise and the `dev-environment` spec instead. | Evidence: `mise.toml`; `scripts/check-mise-lock-platforms.sh` |
| D17 | Does `openspec/discovery.md` change here? | No. `lets-shape` keeps that file, so its next pass updates the stale "Enter env" and "Keep tools current" stages. The pull request notes that they are stale. | Assumption |
| D18 | How is the story accepted? | `git ls-files` lists none of the D1 files; `git grep` finds no Nix setup route outside the history in D2 and the NixOS note in D15; `mise run check` passes; and CI passes on the pull request. | Evidence: #502 body (outcome); `AGENTS.md` (gate) |

## Changes from the story

- Changed: `.envrc.local` stays in `.gitignore`, and so does `.direnv/`, with a comment that they are Nix-route leftovers. The story removed `.envrc.local`. Keeping both stops a former Nix user's personal settings from being committed by accident (D5).
- Added: a `CONTRIBUTING.md` note for contributors moving off the Nix shell, as #496 suggests by naming mise from Nixpkgs (D14).
- Added: the `ci` and `dev-commands` specs drop their Nix wording, as #496 asks those specs to match; the story named only `dev-environment` (D9, D10).
- Added: comments in `mise.toml` and `scripts/check-mise-lock-platforms.sh` stop naming the Nix shell and `flake.nix` (D16).
- Deferred: the stale stages in `openspec/discovery.md` wait for the next `lets-shape` pass (D17). The `ci` spec's older path-filter mismatch stays out too, named in the pull request (D9).

Confirmed by user: no
