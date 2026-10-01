## Context

See proposal.md for why Rust should follow the latest stable release. This section covers only the facts that shape how.

- **The Nix shell reads the toolchain file.** `nix/toolchains.nix` passes `server/rust-toolchain.toml` to `rust-overlay`'s `fromRustupToolchainFile`. `rust-overlay` is a flake input, locked in `flake.lock`. It knows only the Rust releases that existed when its locked commit was made.
- **An unknown release fails the shell.** With today's lock, `channel = "1.99.0"` stops with `error: Stable 1.99.0 is not available`. The flake check and the `v2` server job both build the shell first, so both fail.
- **`stable` resolves through the lock.** With today's lock, `channel = "stable"` resolves to `rust-default-1.98.1`, the newest release that the locked `rust-overlay` knows.
- **The code is ready for 1.98.1.** On 2026-09-30 the author ran the server gates on 1.98.1, built from the locked inputs. Clippy with `-D warnings`, `cargo fmt --check` and `cargo test` all passed with no code changes. Clippy also passed with its minimum supported version set to 1.98.1, which is how it behaves once `rust-version` is gone. The server crates hold 260 lines of Rust.
- **Other files repeat 1.85.** `server/Cargo.toml` sets `rust-version = "1.85"`. `server/Dockerfile` builds from `rust:1.85-slim-bookworm`, and its comment says "pinned 1.85". `server/README.md` states "stable 1.85" twice, and `AGENTS.md` states "Stable Rust 1.85".
- **The server image installs Rust through rustup.** The official `rust` image names its toolchain by release number, such as `1.98.1`, not `stable`. rustup in the image reads the copied toolchain file, so `channel = "stable"` installs a second toolchain. rustup 1.29.1 still does this on its own when `cargo` runs, but warns that this automatic install is deprecated.
- **No CI job builds `server/Dockerfile`.** `docker-build-push.yml` ignores `server/**`, and `docker-build-test.yml` builds the root `Dockerfile`.
- **Dependabot already updates `flake.lock` weekly.** The flake check and the `v2` server job run on that pull request.

## Goals / Non-Goals

**Goals:**

- A new Rust release reaches the repo with no manual step, unless it brings new lints.
- The Nix shell and CI get the exact release from `flake.lock`, so one commit always builds with one release.
- A reviewer of the weekly `flake.lock` pull request can see whether Rust changed.

**Non-Goals:**

- Contributors without Nix getting exactly CI's release. They get the stable release they last installed.
- A reproducible Rust release inside the server image. The image is not published yet.
- Any change to the Dependabot configuration.

## Decisions

### D1. The toolchain file names the `stable` channel

`server/rust-toolchain.toml` sets `channel = "stable"` and keeps the `rustfmt` and `clippy` components. `nix/toolchains.nix` keeps its code, and only its comment changes. The Nix shell then takes the newest stable release that the locked `rust-overlay` knows. The weekly `flake.lock` update moves `rust-overlay`, and Rust with it, in one pull request.

This matches how the repo handles Node. `.nvmrc` names only major version 26, and `flake.lock` picks the exact 26.x.y.

Alternatives considered:

- **Exact pin, bumped by Dependabot's `rust-toolchain` ecosystem.** Dependabot opens the toolchain pull request and the `flake.lock` pull request at the same time. The toolchain pull request then asks for a release that the old `rust-overlay` doesn't know, so its checks fail. Someone would have to merge the `flake.lock` update first and rebase, at almost every Rust release. The maintainer rejected that routine.
- **Exact pin, with both updates in one Dependabot multi-ecosystem group.** This keeps the exact pin and avoids the ordering problem. GitHub's documentation doesn't say whether `nix` or `rust-toolchain` can join such a group, and a GitHub code search found no repository that does it. We would learn whether it works only at the next Rust release.
- **Minor-version channel, such as `1.98`.** Each new minor release still needs a bump to this file, so it has the same ordering problem.
- **Nix ignores the file and uses `rust-bin.stable.latest`.** The Nix shell and rustup would then read two different settings, and could disagree about components.

### D2. `server/Cargo.toml` loses its `rust-version` field

The server is an application, not a published library. The only compiler it supports is the release that `flake.lock` decides, which CI uses. It therefore has no separate minimum version to declare. Clippy reads `rust-version` as that minimum, and skips suggestions that need a newer release. The workspace uses resolver 2, which ignores `rust-version` when it chooses dependency versions.

Alternatives considered:

- **Keep it and raise it with each release.** That adds a second file to edit, which is the manual work this change removes.
- **Keep it at 1.85.** Clippy would go on holding the code to 1.85-era features.

### D3. The server image installs the toolchain that the file names

`server/Dockerfile` builds from `rust:1-slim-bookworm`. The builder copies the workspace, then runs `rustup toolchain install` and `cargo build` in one `RUN` step. Without arguments, `rustup toolchain install` installs the toolchain that `rust-toolchain.toml` names.

The explicit install replaces rustup's deprecated automatic install. Keeping it in the same step as the build means Docker never reuses a toolchain apart from the build it made. A source change therefore always builds with the current stable release. The author built the full image with an explicit install on 2026-09-30: rustup installed stable 1.98.1 with no warning, and the container passed its health check.

Alternatives considered:

- **Rely on rustup's automatic install when `cargo` runs.** It works today, but rustup warns that it is deprecated.
- **Install the toolchain in its own layer, before copying the source.** Docker would then reuse that layer until `rust-toolchain.toml` changes, which saves the download on each source change. But the file keeps saying `stable`, so a cached layer keeps an old release indefinitely. Code that needs the newer release that CI uses would then fail to build on that machine.
- **Delete the toolchain file inside the image and use the image's own toolchain.** That saves the second download. But the image's toolchain has no `clippy` or `rustfmt`, and the image would ignore the file that everything else reads.
- **Keep a release-numbered tag, such as `rust:1.98.1-slim-bookworm`, and let Dependabot's `docker` ecosystem bump it.** The Dockerfile would name a release again, in a second place that can drift.

### D4. The flake check prints `rustc --version`

The flake check already prints the `openspec`, Node and `just` versions inside the shell, so a reviewer can see what a `flake.lock` update changes. After D1, a `flake.lock` update can also change the Rust release, and a new release can bring new lints. The flake check therefore also prints `rustc --version`.

### D5. The docs describe the stable-channel rule

- **`AGENTS.md`:** the "Toolchain is pinned" rule becomes: Rust follows the latest stable release. `server/rust-toolchain.toml` names the `stable` channel, and `flake.lock` decides the exact release in the Nix shell and CI.
- **`CONTRIBUTING.md`, "With Nix":** step 4 stops claiming that the Nix release matches rustup's.
- **`CONTRIBUTING.md`, "Without Nix":** rustup installs the latest stable release, and `rustup update` moves to a newer one. CI can lag behind the latest stable release by up to about two weeks. The flake check log shows CI's release.
- **`CONTRIBUTING.md`, "Reviewing a pin update":** a `flake.lock` update can change the Rust release, and the flake check log shows it. If new clippy lints fail the update, fix them in a separate pull request to `main`, then comment `@dependabot rebase` on the update.
- **`server/README.md`:** both mentions of 1.85 go.

### D6. The change corrects ADR-013 in place, with no new ADR

ADR-013 says that for Rust, the Nix shell and rustup "read the same exact-patch pin file, so they can't drift". D1 makes that statement false. The change rewrites that one consequence in ADR-013, so that it covers Rust and Node together. The maintainer decided that D1 needs no ADR of its own: ADR-009 already chooses Rust for the backend, and which Rust release to use is a toolchain setting under ADR-013.

## Risks / Trade-offs

- **[Contributors without Nix can use a different release from CI]** → A contributor's rustup keeps the stable release they last installed. That can be older or newer than CI's release. CI stays the authority, and `CONTRIBUTING.md` says where to find CI's release. The two routes already differ this way for Node patch releases.
- **[A newer local release can suggest code that CI's release can't compile]** → Without `rust-version`, clippy assumes the local compiler is the minimum. A contributor on a release newer than CI's can get a suggestion that uses a just-stabilized API. CI then fails to compile the pull request. CI catches this before merge, and it can happen only in the week or two before `flake.lock` catches up. Contributors who use the Nix shell never meet it.
- **[A crate update can need a newer compiler than CI's]** → Dependabot's weekly Cargo pull request can include a crate that requires a Rust release newer than the one in `flake.lock`. That pull request then fails until the next `flake.lock` update merges. This needs a crate to require a release that came out in the last week or two, which is rare. With the old fixed 1.85 pin it was more likely.
- **[New lints can block the weekly `flake.lock` pull request]** → A new Rust release can add a clippy lint that fails `-D warnings`. The whole `flake.lock` update, including `nixpkgs` and the `openspec` CLI, then waits until someone fixes the lint on `main`. This needs work only when a release adds a lint that the code triggers, not at every release. The 1.85 to 1.98.1 jump added none.
- **[Rust arrives a week or two late]** → Rust reaches the repo only when the weekly `flake.lock` pull request is merged after `rust-overlay` learns the release. The project wants current Rust, not same-day Rust, so this delay is acceptable.
- **[The server image's Rust release is not reproducible]** → The image builds with whatever release is stable at build time. D3 keeps the toolchain install in the build step, so a cached layer never holds a compiler apart from its build. Every source change downloads the toolchain again, which takes about 10 seconds. The image is not published yet. Pinning the image's Rust belongs to the story that starts publishing it.
- **[The server image downloads Rust without a committed hash]** → rustup checks each download against the SHA-256 in the Rust channel manifest, which it fetches over HTTPS. No hash in the repo covers it. The current Dockerfile already downloads Rust this way. The dev-environment requirement on verifying remote code covers the Nix shell and `.envrc`, not the server image.
- **[No CI job builds the server image]** → A broken `server/Dockerfile` would go unnoticed. The tasks include a manual `docker build server/`. A CI job for the image is out of scope.
- **[Each Rust release costs one cold compile]** → Cargo rebuilds `server/target` when the compiler changes. `Swatinem/rust-cache` keys on the Rust release, so the `v2` server job also misses its cache once per release.

## Migration Plan

1. Merge this change. The next shell load downloads Rust 1.98.1, and cargo rebuilds `server/target` once.
2. Contributors without Nix run `rustup update` to leave 1.85.
3. To roll back, revert the merge commit. The exact 1.85.0 pin and `rust-version` return with it.
