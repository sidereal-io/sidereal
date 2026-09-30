---
id: adrs-adr014
date: 2026-09-30
status: proposed
title: 'ADR014: Rust Follows the Latest Stable Release'
description: Architecture Decision Record (ADR) for how Sidereal chooses its Rust compiler release — the stable channel, with the exact release decided by the Nix lock file.
---

# ADR-014: Rust Follows the Latest Stable Release

## Context

Sidereal's backend is written in Rust. Rust publishes a new stable compiler release every six weeks. Each release can add new lint warnings, and the project's checks treat every lint warning as an error.

The project pins its development tools with a Nix flake, as ADR-013 describes. A flake records the exact version of every input in a committed lock file. Rust comes from one of those inputs: a catalogue of Rust releases that knows only the releases published before its locked version.

Rust also has a pin file of its own, `rust-toolchain.toml`. Both the Nix flake and rustup, Rust's usual installer, read it. Until now it named one exact release. Nothing moved that release forward, so the project fell 13 releases behind.

An automated updater, Dependabot, opens update pull requests once a week, and it can bump that exact release. But the bump would ask for a release that the locked catalogue doesn't know yet. Its checks would fail until a maintainer merged the lock file update first and rebased the bump. That manual step would recur at almost every Rust release.

## Decision

`rust-toolchain.toml` names the `stable` channel instead of an exact release. In the Nix shell and in CI, the lock file decides the exact release: the newest stable release that the locked catalogue knows. New Rust releases arrive with the weekly lock file update, in one pull request.

No other file names a Rust release. The server declares no minimum supported Rust version. The only compiler it supports is the release that the lock file decides, which CI uses.

## Consequences

- **Rust stays current without a manual step.** A new release arrives with the next weekly lock file update.
- **One commit still means one compiler release** in the Nix shell and in CI, because the lock file decides it.
- **Contributors without Nix can use a different release from CI.** rustup keeps whichever stable release they last installed, until they run `rustup update`. This replaces ADR-013's statement that the Nix shell and rustup "read the same exact-patch pin file, so they can't drift". Rust now drifts between those two routes the way Node patch releases already could.
- **A release that adds a lint the code triggers holds back the whole lock file update.** That update also carries the other tools, so they wait until someone fixes the lint.
- **Rust arrives up to about two weeks late.** It waits for the catalogue to learn the release, and then for the weekly update to be merged.
- **The server image gets whatever release is stable when it is built.** Its Rust release is not reproducible until the project decides how to pin it for published images.

## Alternatives Considered

### Alternative 1: An exact release, bumped by Dependabot

- **Pros**: every route, including rustup and the server image, uses exactly the same release.
- **Cons**: the bump fails until the lock file update merges first, at almost every Rust release.
- **Why not**: it turns every Rust release into manual merge-order work for the maintainer.

### Alternative 2: An exact release, with both updates in one Dependabot pull request

- **Pros**: the exact release stays, and the release and the catalogue move together, so the checks pass.
- **Cons**: Dependabot's documentation doesn't say whether it can group these two kinds of update, and no known project does it.
- **Why not**: the project would learn whether it works only at the next Rust release, and would carry extra configuration for this one case.

### Alternative 3: A minor-version channel, such as `1.98`

- **Pros**: patch releases arrive on their own.
- **Cons**: each new minor release still needs a bump to the pin file, with the same merge-order problem as Alternative 1.
- **Why not**: it solves only the least important part of the problem.
