---
id: adrs-adr016
date: 2026-11-19
status: accepted
title: 'ADR-016: Development Environment with mise'
description: Architecture Decision Record (ADR) for how Sidereal pins its development tools — mise with a committed lock file, in place of the Nix flake, with the plain pin files kept for contributors who use neither.
---

# ADR-016: Development Environment with mise

## Context

Sidereal pins its development tools so that contributors, AI agents and CI build with the same releases. The tools are Rust, Node, pnpm, a task runner, the GitHub CLI, the OpenSpec CLI, and Let's Spec, which copies the agents' skills into the repo. [ADR-013](ADR-013-development-environment.md) chose a Nix flake for this, and turned mise down because mise could not reproduce an install exactly.

Four things have changed since then:

- **Let's Spec now ships a mise route.** Its skills and the OpenSpec CLI no longer need the flake.
- **mise now writes a lock file.** `mise.lock` records each tool's exact release, with a download address and checksum for each platform. That answers the reproducibility objection.
- **The flake is hard to maintain.** Changing it needs the Nix language and flake-parts, which most maintainers do not know.
- **Automated updates to the flake fail.** When Dependabot updates the flake's lock file, the committed skills no longer match the new Let's Spec release. The check that compares them fails until someone enters the shell by hand and commits what Let's Spec wrote.

## Decision

Sidereal pins its development tools with **mise**. One `mise.toml` at the repo root names each tool, and a committed `mise.lock` pins its exact release. `mise activate` turns the environment on, and its `enter` hook keeps the skills current. CI installs from the same lock. The Nix flake is retired.

The plain pin files stay: `server/rust-toolchain.toml` for rustup and `.nvmrc` for Node version managers. Contributors who use neither mise nor Nix install by hand from them.

## Consequences

- **One lock file pins every tool except Rust,** on contributors' machines and in CI, for Linux and macOS alike.
- **Rust follows the stable channel, not a pinned release.** `mise.toml` and `server/rust-toolchain.toml` both name `stable`, and `mise.lock` records the channel. rustup's newest stable release on each machine decides the release when mise installs it. Two machines can therefore build with different Rust releases, as two rustup users can today. No file names a Rust release.
- **Maintainers edit TOML,** not the Nix language.
- **A tool update can arrive as one pull request that passes on its own:** a workflow can update the lock and run Let's Spec's sync in the same step. Dependabot cannot, because it has no mise support.
- **Isolation is weaker than Nix's.** mise installs tools into its own directories, not an isolated store. System libraries, such as OpenSSL, come from the machine, so contributors install them themselves.
- **Contributors need mise,** and must run `mise activate` in their shell for the `enter` hook to run. Agents and CI that do not activate mise run tools through `mise exec`; there the hook does not run, and the committed skills cover it.
- **Entering the repo runs code from the working tree.** mise asks for trust once per checkout path, not per change, so a contributor who checks out an untrusted branch must withdraw trust first.
- **Two routes exist until the flake is removed.** Tools that both lock files pin must move in both, together.

## Alternatives Considered

### Alternative 1: Keep the Nix flake

- **Pros:** the strongest reproducibility, including system libraries. Already working.
- **Cons:** needs Nix and flake-parts knowledge to change. Its automated updates fail until someone runs the sync by hand.
- **Why not:** the two costs fall on every maintainer and every update, and mise's lock file now gives the reproducibility the project needs from its tools.

### Alternative 2: devbox or devenv

- **Pros:** friendlier configuration than a raw flake. Both build on Nix's store.
- **Cons:** both still need Nix installed, and their updates still change a Nix lock file, so the sync problem stays.
- **Why not:** they keep Nix underneath, which is the dependency this decision removes.

### Alternative 3: asdf

- **Pros:** reads the same per-tool version files, and many plugins exist.
- **Cons:** no lock file with checksums, and plugins are shell scripts each maintained separately. Let's Spec ships no asdf route.
- **Why not:** it pins versions without verifying downloads, which brings back the reproducibility gap.

### Alternative 4: No tool manager; install by hand from the pin files

- **Pros:** nothing to install beyond the tools themselves.
- **Cons:** the pin files name only a channel or a major version, so every machine lands on a different release, and nothing keeps the skills current.
- **Why not:** contributors and agents would build with different tools again. The route stays available for those who choose it.
