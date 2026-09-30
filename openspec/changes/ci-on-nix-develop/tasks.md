## 1. Split the recipe (design D1, D2)

- [x] 1.1 In the `justfile`, move the body of `check` into a new `check-server` recipe, unchanged, and make `check` depend on it with no body. Give each recipe a one-line comment, since `just --list` prints it. Verify `just --show check` contains the line `check: check-server`, and `just --list` shows both recipes.
- [x] 1.2 Run `just check` and `just check-server`, and verify both exit with status 0 and run the same four checks. Commit the `justfile`.

## 2. Move the `server` job into the shell (design D3–D6)

- [x] 2.1 In `.github/workflows/v2.yml`, add `flake.nix`, `flake.lock`, and `nix/**` to the `pull_request` paths. Verify `yq '.on.pull_request.paths' .github/workflows/v2.yml` lists six entries. Run `yq` through `nix shell nixpkgs#yq-go` if it is not installed.
- [x] 2.2 Remove the job's `defaults` block, the `rustup show` step, and the five check steps. Add the `Install Nix` step from `nix.yml` after checkout. Verify `grep -n rustup .github/workflows/v2.yml` finds no match.
- [x] 2.3 Move the rust-cache step to after `Install Nix`, keep `workspaces: server`, and add `cmd-format: nix develop -c {0}`. Verify the spec scenario "The cache step uses the shell's Rust" prints `nix develop -c {0}`.
- [x] 2.4 Add one last step that runs `nix develop --command just check-server`. Verify the spec scenarios "The server job has one check step" and "The server job has no step that installs a tool" print the output they state.
- [x] 2.5 Run `actionlint .github/workflows/v2.yml` (`nix shell nixpkgs#actionlint` if it is not installed), and verify it reports no error. Commit the workflow.

## 3. Contributor docs

- [x] 3.1 In `server/README.md`, add a `just check-server` row to the Recipes table, and say that `just check` runs it. Verify the table names both recipes.
- [x] 3.2 In `CONTRIBUTING.md`, under "Reviewing a pin update", say that a Nix pull request now also runs the `server` job with the new pins. Verify the section names both the flake check and the `server` job.
- [x] 3.3 In `openspec/discovery.md`, update the Open PR stage: v2 CI now runs inside the pinned shell, so drop that gap and its #289 link. Keep the web gap and #301. Verify `grep -n '289' openspec/discovery.md` finds no match in the Open PR stage. Commit the docs.

## 4. Verify the whole change

- [x] 4.1 Run `nix develop --command just check-server`, and verify it exits with status 0.
- [x] 4.2 Prove the gate fails: add a blank line inside a function in one Rust file, run `nix develop --command just check-server`, and verify it exits with a non-zero status at `cargo fmt --check`. Then restore the file and verify `git status --porcelain` prints nothing.
- [x] 4.3 Run `openspec validate ci-on-nix-develop --strict`, and verify it passes.
- [ ] 4.4 On the pull request, verify the `v2 / server` job and the `nix` flake check both run and pass. In the rust-cache step's log, verify it loaded the shell from `server/` and that no line reports rustup downloading a toolchain. If the rust-cache step fails, stop and agree a new D5 with the maintainer (design.md, Risks).
- [ ] 4.5 Record three things in the pull request description: the `server` job's run time, the rust-cache step's run time, and that `ci.yml` also ran because this pull request changes the root `justfile` (#322).
