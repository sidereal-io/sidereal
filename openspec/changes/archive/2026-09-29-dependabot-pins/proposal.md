## Why

Nothing updates `flake.lock` or `backend/Cargo.lock` today, so the Nix and Cargo pins drift until someone updates them by hand. No CI job builds the Nix shell either, so a bad lock update or flake edit breaks only when a contributor's shell fails to load.

## What Changes

- Dependabot opens a weekly pull request that updates the `flake.lock` inputs. The inputs are `nixpkgs`, `rust-overlay`, and `flake-parts`.
- Dependabot opens a weekly pull request that updates the crates in `backend/Cargo.lock`.
- The configuration asks Dependabot for one grouped pull request per ecosystem. If Dependabot ignores the group for Nix, we accept one pull request per input.
- A new CI job, the flake check, runs `nix flake check` on every pull request that changes a file that defines the shell. It then runs `openspec`, Node, and `just` inside the shell and prints their versions. The job fails if any of them fails.
- `CONTRIBUTING.md` gains a short section on reviewing a pin update.

The Rust toolchain version stays out of scope; story #310 handles it. The Node major version, moving the existing CI jobs onto Nix (#289), and a binary cache (#288) also stay out.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds requirements for automatic pin updates and for a CI check that builds the shell on every pull request that changes it.

## Impact

- **Changed files**: `.github/dependabot.yml` gains two entries; a new workflow file for the flake check goes in `.github/workflows/`; `CONTRIBUTING.md` gains one section.
- **Unchanged**: no application code, no flake module, and no existing CI job changes.
- **Reviewer load**: Mo receives up to two routine pull requests a week, or up to four if Dependabot does not group Nix inputs.
- **Secrets**: none. Dependabot pull requests trigger CI with a read-only token, and the new job needs no secret.
- **Cargo path**: the `cargo` entry names `/backend`. Story #299 renames that directory and will update the entry.
