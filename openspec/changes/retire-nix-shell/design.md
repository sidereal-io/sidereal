<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

Every CI job and every development task already runs through mise. The Nix route is now only:

- the files `flake.nix`, `flake.lock`, `nix/devshell.nix`, `nix/lets-spec.nix`, `nix/openspec.nix`, `nix/toolchains.nix` and `.envrc`;
- the `nix` workflow (`.github/workflows/nix.yml`), whose `flake check` job no ruleset requires (D4);
- the `nix` entry in `.github/dependabot.yml`;
- text in the docs, the specs and two code comments.

`git grep -n -i -E 'nix|flake|direnv'`, outside the archive and the decision records, finds every place that names it. The hits outside the files to delete are:

| File | What names the Nix route |
|---|---|
| `AGENTS.md` | The Rust durable constraint (`flake.lock` decides the release); "Change a version only where it is set" (both locks); "Update Let's Spec on both routes" |
| `CONTRIBUTING.md` | The "Development Environment" intro; "With Nix"; the heading "Without Nix" and four links to `#without-nix`; step 5 of that section ("Only the Nix shell and mise"); the `direnv deny` warning; "Where versions are set" (intro, OpenSSL, Rust, Node, pnpm, Let's Spec and `openspec` rows, the Rust note, the Dependabot paragraph); "Agent skills" (sync on load, the `flake.nix` tag, the by-hand steps) |
| `README.md` | "Get set up" (With Nix / Without Nix) and "Without mise, including inside the Nix shell" |
| `server/README.md` | "The optional Nix shell provides the build tools" and a link to `#without-nix` |
| `web/README.md` | "The Nix shell provides both" and "rather than taking one from Nix" |
| `docs/dependency-updates.md` | "`flake.nix` alone … the Nix route is still a manual step" and "The Nix workflow still uses path filters" |
| `.gitignore` | The comment over `.direnv/` and `.envrc.local` |
| `mise.toml` | "Keeps the OpenSpec CLI offline, as the Nix shell does." |
| `scripts/check-mise-lock-platforms.sh` | "The systems flake.nix supports." and "each system the dev shell supports" |
| `openspec/specs/dev-environment/spec.md`, `openspec/specs/ci/spec.md` | The Purpose paragraphs, which a delta cannot change |

`openspec/discovery.md` also names the Nix route. It stays as it is (D17).

OpenSSL is not pinned on the mise route. `server/scripts/db-tls.sh` and `scripts/external-db-verify.sh` call `openssl` from the host.

The disk on the development machine has about 9 GB free. Nothing in this change builds anything new.

## Goals / Non-Goals

**Goals:**

- After the change, the repo describes and supports two routes: mise, and tools installed by hand ("without mise").
- Every link to a renamed section still lands on a heading.
- A contributor who used the Nix shell can find, in `CONTRIBUTING.md`, what to undo and where to go next (D14).
- No tool version moves. `mise.lock` and `mise.toml`'s tool lines stay byte for byte the same.

**Non-Goals:**

- Uninstalling Nix or direnv from any machine.
- Rewriting history: archived changes, ADR-013 and ADR-016 keep their Nix text (D2).
- Fixing the `ci` spec's older path-filter mismatch (D9). The pull request names it.
- Updating `openspec/discovery.md` (D17).

## Test Seams

This change deletes files and edits text. It adds no code, so there is no unit to test. Each spec scenario is a shell command with a fixed expected output, and those commands are the tests.

- **Primary test seam:** shell checks run at the repo root, one per spec scenario:
  - `git ls-files flake.nix flake.lock nix .envrc .github/workflows/nix.yml` prints nothing;
  - `yq '.updates[] | select(."package-ecosystem" == "nix")' .github/dependabot.yml` prints nothing;
  - with a scratch `.direnv/` and `.envrc.local`, `git status --porcelain` lists neither;
  - `git grep -n -i -E 'nix|flake|direnv|envrc'` outside the allowed history finds only the hits that the "Allowed hits" decision lists.

  Each check fails before its task and passes after it.
- **Existing seams reused:** `mise run check`. Its `check-openspec` part validates the deltas and the edited Purpose paragraphs. The `ci` workflow's `lets-spec` job runs `lets sync --check`, which proves the `AGENTS.md` rules block between its markers did not change. CI on the pull request proves that removing the `nix` workflow leaves the required checks passing.

## Scrutiny Tier & Blast Radius

- **Tier:** medium. Many files change, but on one subsystem, the development environment, and with no new contract. The one behaviour change, removing the Nix route, breaks a workflow for a known group: contributors on the Nix shell.
- **Blast radius:** the development setup docs, `AGENTS.md`'s project sections, Dependabot's configuration, the CI workflows list, and three specs. `server/` and `web/` code do not change, and nothing that ships in the app image changes.

## Decisions

### Delete the Nix route in one commit, with no deprecation period

Delete the eight files and the Dependabot entry together, in one `chore:` commit. ADR-016 already decides the retirement (D3). Every task already runs without Nix. A deprecation period would keep two routes alive with nothing to gain.

- Alternative: keep `flake.nix` as an unsupported extra. Rejected by the story's scope (D2), and it would go stale silently.

### Name the second route "Without mise"

Rename `CONTRIBUTING.md`'s "Without Nix" section to "Without mise", so its anchor becomes `#without-mise`. Change every link to `#without-nix`: two in `CONTRIBUTING.md` itself and one in `server/README.md`. The `dev-commands` spec already says "for contributors without mise" after this change (D10), so the heading matches the spec.

- Alternative: "By hand". Rejected: the spec scenarios and the `README.md` bullets already say "without mise".

### Put the Nix move note after "Without mise"

Add a short subsection, "Moving from the Nix shell", at the end of the routes, before "Where versions are set". It lists, in order:

1. run `direnv revoke` in the repo root, so direnv stops loading `.envrc`;
2. delete `.direnv/`;
3. move any settings in `.envrc.local` to `mise.local.toml`;
4. follow [With mise](#with-mise). mise is also in Nixpkgs, so a Nix user can install it from there.

`.direnv/` and `.envrc.local` stay in `.gitignore` (D5). Their comment changes to say they are leftovers from the retired Nix route.

### List OpenSSL as a system prerequisite, not a pinned tool

Remove the OpenSSL row from "Where versions are set": no file sets its version any more. Name OpenSSL instead in the paragraph after the "Without mise" steps that lists what the gate and the tasks need from the system. It already names Docker. Say there that `db-tls` and `external-db-verify` need `openssl` on `PATH`, on both routes (D13).

### Keep the spec renames as REMOVED plus ADDED

OpenSpec 2.1.1 refuses a MODIFIED block that drops a scenario the main spec has. So each requirement that loses a Nix scenario is removed and re-added under a new name, with every surviving scenario:

| Old requirement | New requirement |
|---|---|
| `ci`: "The server checks run when server code changes" | "The server checks run when server or web code changes" |
| `dev-environment`: "Rust follows the latest stable release" | "Rust follows the stable channel" |
| `dev-environment`: "Nix stays optional" | "Contributors can work without mise", and "mise loads no application settings" |
| `dev-environment`: "Both routes use one Let's Spec release" | "The committed skills match the pinned Let's Spec release" |

Two further requirements are new: "The repo has no Nix route" and "Leftover Nix-route files stay out of git". They state the D18 acceptance and D5 as checks that outlive this change. Requirements that keep all their scenarios use MODIFIED.

- Alternative: RENAMED plus MODIFIED. Rejected: the MODIFIED block would still have to carry the dropped scenarios.

### Edit the two Purpose paragraphs in the main specs

A delta cannot change a Purpose, so the build edits the main specs directly:

- `dev-environment`: the Purpose says mise gives every contributor and agent the same pinned tools, that mise stays optional because the plain pin files let a contributor install by hand, and that the skills are the same on every machine. Its `yq` note says mise does not provide `yq`.
- `ci`: its `yq` note says mise does not provide `yq`, in place of "The development shell does not provide it".

### Rewrite the AGENTS.md project sections, never the rules block

Change only the durable constraints and "Working in this repo":

- **Rust follows the latest stable release.** Becomes: `server/rust-toolchain.toml` and `mise.toml` both name the `stable` channel, and rustup decides the release in CI and on contributors' machines.
- **Change a version only where it is set.** Drops the sentence about two locks.
- **Update Let's Spec on both routes.** Becomes **Update Let's Spec through mise.** The weekly `mise-tool-updates.yml` workflow moves it. By hand: change the version in `mise.toml`, leave OpenSpec alone, run `mise lock` and `mise exec -- lets sync`, and commit what it wrote with `mise.toml` and `mise.lock` in one pull request.

The rules block between the lets-spec markers stays byte for byte the same. The `lets-spec` job's `lets sync --check` proves it.

### Point the code comments at mise

- `mise.toml`: "Keeps the OpenSpec CLI offline." with no comparison.
- `scripts/check-mise-lock-platforms.sh`: the supported systems are the ones the `dev-environment` spec names, in "The mise lock pins every other tool's exact release". The header says "each supported system" in place of "each system the dev shell supports".

### Allowed hits after the change

After the change, `git grep -n -i -E 'nix|flake|direnv|envrc'` may still find:

- `openspec/changes/archive/` and `docs/decisions/` (history, D2);
- `openspec/changes/retire-nix-shell/` (this change, until archive moves it);
- `openspec/discovery.md` (D17);
- `.gitignore`'s leftover entries and their comment (D5);
- `CONTRIBUTING.md`'s "Moving from the Nix shell" subsection (D14);
- the NixOS note in `web/README.md` (D15);
- the main specs under `openspec/specs/`: until archive applies the deltas, they keep their Nix requirements. After archive, only "The repo has no Nix route" and "Leftover Nix-route files stay out of git" name the retired route;
- words that only contain the letters, such as "Unix" or "flake" meaning a flaky test (`web/playwright.config.ts`).

## Risks / Trade-offs

- [A contributor on the Nix shell pulls and direnv errors on the missing `.envrc` or flake] → The move note says to run `direnv revoke` first. The pull request description repeats it.
- [A stale link to `#without-nix` or `#with-nix` remains] → A task greps for both anchors across the repo, outside the history, and expects no match.
- [Deleting `nix.yml` leaves a stale required check] → D4 checked the ruleset: `flake check` is not required. CI on the pull request confirms that the required checks report.
- [An edit to `AGENTS.md` touches the rules block by accident] → `lets sync --check` fails in the gate and in CI.
- [Dropping the offline `lets sync` promise (D7) loses a guarantee] → Accepted: `OPENSPEC_NO_UPDATE_CHECK` stays set through mise, and `mise.lock` checks every download.

## Migration & Rollback

No data or deployed system changes shape. Contributors on the Nix shell follow the move note in `CONTRIBUTING.md`. To roll back, revert the pull request's commits: the deleted files come back from git history, and nothing outside the repo needs undoing.
