## 1. Lighter development profile

- [ ] 1.1 [easy] A dev or test build compiles the workspace crates with `-C debuginfo=line-tables-only` and every dependency without debug information, set by `[profile.dev]` (`debug = "line-tables-only"`) and `[profile.dev.package."*"]` (`debug = false`) in `server/Cargo.toml`, with no other `[profile` table (design decision 1) (verify: `cd server && test "$(grep -c '^\[profile' Cargo.toml)" = 2 && cargo clean -p sidereal-plugin-abi && cargo test --no-run -v -p sidereal-plugin-abi 2>&1 | grep -q 'crate-name sidereal_plugin_abi .*debuginfo=line-tables-only'`)

## 2. Size-limited clean-up task

- [ ] 2.1 [medium] `sh server/scripts/target-clean.sh` deletes the repo's `server/target/` when it is larger than `SIDEREAL_TARGET_LIMIT_MIB` MiB and keeps it at or under the limit, printing the size, the limit and which it did, with exit status 0; write the over-limit and under-limit cases first in a new `server/scripts/target-clean-check.sh`, which builds each case in a `mktemp -d` copy of the repo layout (design decision 5, test seam) (verify: `sh server/scripts/target-clean-check.sh`)
- [ ] 2.2 [easy] With no `server/target/` folder, the clean-up script prints that there is nothing to delete and exits with status 0 (verify: `sh server/scripts/target-clean-check.sh`)
- [ ] 2.3 [easy] With no `SIDEREAL_TARGET_LIMIT_MIB` set, the clean-up script uses a limit of 4096 MiB; with a value that is not a whole number, such as `2G`, it deletes nothing, names the variable in its error output and exits non-zero (verify: `sh server/scripts/target-clean-check.sh`)
- [ ] 2.4 [medium] The clean-up script touches only the repo's own `server/target`: it ignores `CARGO_TARGET_DIR`, never calls `cargo clean`, and when `server/target` is a symbolic link it deletes nothing and exits non-zero (verify: `sh server/scripts/target-clean-check.sh`)
- [ ] 2.5 [easy] `mise run target-clean` runs `sh server/scripts/target-clean.sh` from a `[tasks.target-clean]` entry in `mise.toml` with a description, and the `check` task does not run it (verify: `mise tasks ls --no-header | grep -q '^target-clean ' && test "$(mise tasks info target-clean --json | jq -r '.run[]')" = 'sh server/scripts/target-clean.sh' && ! mise tasks info check --json | grep -q target-clean`)

## 3. Contributor documentation [depends-on: 2]

- [ ] 3.1 [easy] `CONTRIBUTING.md` says why the dev profile is lighter and gives the one command line that restores full debug information, `cargo --config 'profile.dev.debug="full"' --config 'profile.dev.package."*".debug="full"' <command>`, which changes no committed file (design decision 2) (verify: `cd server && cargo clean -p sidereal-plugin-abi && cargo --config 'profile.dev.debug="full"' --config 'profile.dev.package."*".debug="full"' test --no-run -v -p sidereal-plugin-abi 2>&1 | grep -q 'crate-name sidereal_plugin_abi .*debuginfo=2' && cd .. && grep -qF "profile.dev.package.\"*\".debug=\"full\"" CONTRIBUTING.md && test -z "$(git status --porcelain)"`)
- [ ] 3.2 [easy] `CONTRIBUTING.md` lists `mise run target-clean` under Development Commands, explains the 4096 MiB limit and `SIDEREAL_TARGET_LIMIT_MIB`, says to run it only when no build runs in that worktree, and adds its row to the task table in step 7 of Without mise (verify: `grep -qF "| \`target-clean\` | \`$(mise tasks info target-clean --json | jq -r '.run[]')\` |" CONTRIBUTING.md && grep -q 'SIDEREAL_TARGET_LIMIT_MIB' CONTRIBUTING.md`)
- [ ] 3.3 [easy] `CONTRIBUTING.md` describes two personal options and their trade-offs, neither set by the project: one build folder shared by all worktrees (`CARGO_TARGET_DIR`; builds wait for each other's lock and rebuild across branches), and sccache (`RUSTC_WRAPPER=sccache` with `SCCACHE_CACHE_SIZE`; faster first builds, no disk saved) (design decisions 3 and 4) (verify: `grep -q 'CARGO_TARGET_DIR' CONTRIBUTING.md && grep -q 'SCCACHE_CACHE_SIZE' CONTRIBUTING.md`)

## 4. Integration [depends-on: 3]

- [ ] 4.1 [medium] The gate passes with the new profile, and the server binary still builds for `dev` and `e2e`, with the PostgreSQL fixture up and `TEST_DATABASE_URL` set (verify: `mise run check && cargo build -p sidereal-server --manifest-path server/Cargo.toml`)
- [ ] 4.2 [easy] The pull request description records the size of `server/target/` from an empty folder after `cargo clippy --all-targets -- -D warnings` then `cargo test --no-run` in `server/`, before and after the profile, with the commands, and the sccache decision with its numbers from design.md (verify: `gh pr view --json body -q .body | grep -q 'cargo test --no-run'`)

## Deviations Ledger
