## 1. Dependabot entries

- [x] 1.1 Add a `nix` entry to `.github/dependabot.yml`: directory `/`, weekly on Monday at 04:00, and a group whose `patterns` list is `["*"]`. Verify that the `yq` command in the scenario "The configuration declares the Nix updates" prints exactly one matching entry.
- [x] 1.2 Add a `cargo` entry to `.github/dependabot.yml`: directory `/backend`, weekly on Monday at 04:00, and a group whose `patterns` list is `["*"]`. Verify that the `yq` command in the scenario "The configuration declares the Cargo updates" prints exactly one matching entry.
- [ ] 1.3 Validate the whole file against the Dependabot schema with `uv run --with check-jsonschema check-jsonschema --builtin-schema vendor.dependabot .github/dependabot.yml`. Verify that it exits with status 0.

## 2. The flake check workflow

- [x] 2.1 Create `.github/workflows/nix.yml` with one job, `flake-check`. It runs on `pull_request` events that change `flake.nix`, `flake.lock`, `nix/**`, `backend/rust-toolchain.toml`, or `.github/workflows/nix.yml`. It sets `permissions: contents: read` and uses no secrets. Verify with `grep -n 'secrets\.' .github/workflows/nix.yml`, which must print nothing.
- [x] 2.2 Add the job's steps:
  1. check out the repo;
  2. install Nix with `DeterminateSystems/determinate-nix-action@v3`;
  3. run `nix flake check`;
  4. run `nix develop --command sh -c 'openspec --version && node --version && just --version'`.

  Verify locally that steps 3 and 4 exit with status 0.
- [x] 2.3 Run the step 4 command with `openspec` replaced by `false`. Verify that the step exits with a non-zero status, as the scenario "A pin update breaks a tool" requires.

## 3. Review guidance

- [ ] 3.1 Add a "Reviewing a pin update" section to the Development Environment part of `CONTRIBUTING.md`, as design D7 describes. Verify that the section names the flake check, the `openspec` release notes, and the existing Rust workflow.

## 4. Verification on GitHub

- [ ] 4.1 On this change's pull request, verify that the flake check runs, passes, and prints the three versions in its job log. This pull request changes the workflow file, so it triggers the check.
- [ ] 4.2 Push a throwaway commit that breaks `flake.nix` to a scratch branch, and open a draft pull request from it. Verify that the flake check fails. Then close that pull request and delete the branch.
- [ ] 4.3 Dependabot reads its configuration only from `main`, so the Dependabot checks can run only after merge. Add an "After merge" checklist to this change's pull request description, and verify that it names these four checks:
  1. open the repo's Dependabot status page (Insights, then Dependency graph, then Dependabot) and confirm that the `nix` and `cargo` entries show no configuration error;
  2. if Dependabot reports the Nix `groups` block as invalid, remove it in a follow-up pull request, as design Risks describes;
  3. after the first weekly run, confirm that the Nix and Cargo pull requests match the scenarios "A flake input has a newer commit" and "Several crates have newer releases";
  4. confirm that the flake check ran on the Nix pull request, and record the outcome on issue #287.
