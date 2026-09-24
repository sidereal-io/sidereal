---
id: adrs-adr013
date: 2026-09-24
status: proposed
title: 'ADR013: Development Environment'
description: Architecture Decision Record (ADR) for how Sidereal pins its development tools — a native Nix flake built from small modules, turned on by direnv, and optional for contributors.
---

# ADR-013: Development Environment

## Context

Sidereal is one repository with two stacks: a TypeScript app built with Node, and a Rust backend. Contributors and AI coding agents both work in it. Each of them builds the repo with whatever tools their own machine has installed.

Tool versions drift as a result. Only Rust has a pin file. The Node version differs between the repo's own files, CI and contributors' machines. The OpenSpec CLI, which generates the agents' workflow instructions, is not pinned at all.

We want one environment that pins every tool and turns on when anyone enters the repo. An agent should get the right tools without having to work anything out.

Two constraints shape the choice:

- **Open source.** The project must not depend on any hosted service to build.
- **Reuse.** We may later share part of this setup between repositories, so it must split into modules that don't depend on any one vendor.

## Decision

Sidereal pins its development tools with a **native Nix flake**, built from small flake-parts modules. direnv turns the flake's shell on when you enter the repo.

**Nix is optional.** The repo keeps the plain pin files that rustup and Node version managers read, so contributors without Nix can still build and check everything.

## Consequences

- **Every tool version comes from one committed lock file.** Tools change only when someone updates that file in a pull request.
- **Contributors with Nix need two tools:** Nix and direnv. There is no wrapper tool between them and the flake.
- **CI can use the standard `nix flake check` command** to hold environment checks.
- **The setup splits into modules.** A module with nothing specific to Sidereal can move into a shared flake later without changes.
- **Editing the environment requires Nix knowledge.** The Nix language and flake-parts are harder to learn than friendlier wrappers. Most contributors only enter the shell, but maintainers carry this cost.
- **Services cost more setup.** Running a database for development needs an extra flake module, where some wrappers offer this built in.
- **Two routes must stay working:** the Nix shell and the plain pin files. They can drift apart, because Nix pins exact patch versions while the plain files can agree with it only on major versions.

## Alternatives Considered

### Alternative 1: devenv

- **Pros:** friendlier options, clearer errors, and built-in services and startup tasks.
- **Cons:** every contributor and every CI job must install the devenv tool as well as Nix. Its plain flake mode supports fewer features. Shared modules would require devenv in every repository that uses them.
- **Why not:** it trades a smaller learning curve for editors against a permanent extra dependency for everyone, including those who only enter the shell.

### Alternative 2: mise or devbox

- **Pros:** simple configuration and a small learning curve.
- **Cons:** mise installs tools outside Nix's isolated store, so builds are less reproducible. Neither offers a module system as strong as flake-parts for sharing setup between repositories.
- **Why not:** reproducibility and sharing are the two goals this decision exists for.

### Alternative 3: flox

- **Pros:** built on Nix, with a friendly interface.
- **Cons:** its sharing model centres on a hosted service.
- **Why not:** an open-source project must not depend on a hosted service to build.

### Alternative 4: Nix required for everyone

- **Pros:** one route to maintain, and no drift between routes.
- **Cons:** it forces every contributor to install and learn to use Nix before their first change.
- **Why not:** the friction would deter occasional contributors, and the plain pin files cost little to keep.
