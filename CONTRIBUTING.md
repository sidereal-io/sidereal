# Contributing to Sidereal

Thank you for your interest in contributing to Sidereal! We welcome contributions from the community to help make this astrophotography management tool even better.

## 🚀 Quick Start

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/sidereal.git
   cd sidereal
   ```
3. **Set up** the tools. See [Development Environment](#-development-environment).
4. **Create** a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```
5. **Make** your changes, and run `mise run check`
6. **Submit** a pull request

### Fixing the running v0.10.x app

`main` holds the rebuild of Sidereal. The released app, v0.10.x, lives on the
[`v0.x` branch](https://github.com/sidereal-io/sidereal/tree/v0.x). To fix it,
branch from `v0.x` and open your pull request against `v0.x`. Its own
`CONTRIBUTING.md` explains its setup. `main` takes no v0 code.

## 📋 Types of Contributions

We welcome several types of contributions:

### 🐛 Bug Reports
- Use the GitHub issue tracker
- Include detailed reproduction steps
- Say which Sidereal version you run, and provide environment information (OS, browser, etc.)
- Include logs or screenshots if applicable

### 💡 Feature Requests
- Use GitHub Discussions for feature ideas
- Describe the problem you're trying to solve
- Explain your proposed solution
- Consider the impact on existing users

### 🔧 Code Contributions
- Bug fixes
- New features
- Performance improvements
- Documentation improvements
- Test coverage improvements

### 📖 Documentation
- README improvements
- Code comments
- API documentation
- Deployment guides
- Troubleshooting guides

## 🛠️ Development Commands

The tasks in `mise.toml` run the server, the web interface, the database fixture
and the gate. Run each one as `mise run <task>` from the repo root.
`mise tasks` describes every task.

```bash
mise run db-up          # Start the isolated PostgreSQL fixture (Docker, Compose and OpenSSL required)
export DATABASE_URL=$(mise run db-url)
export TEST_DATABASE_URL=$(mise run db-test-url)
mise run dev            # Run the server and the web interface together; Ctrl-C stops both
mise run server         # Run the server only
mise run web            # Run the web interface only
mise run e2e            # Run the Playwright end-to-end suite (the fixture must be up)
mise run db-schema      # Regenerate the normalized schema descriptions
mise run db-down        # Stop the fixture, keeping its data
mise run db-clean       # Delete the fixture's containers and data
mise run db-test-clean  # Remove databases left by killed tests
mise run db-tls         # Generate local fixture certificates (OpenSSL required)
mise run check           # The gate to pass before every pull request
mise run check-compose   # Compose configuration checks (Compose CLI + jq; no daemon)
mise run check-openspec  # OpenSpec checks: strict validation of every spec and active change
mise run check-server    # Server checks: format, clippy, tests, dependency-direction lint
mise run check-web       # Web checks: types, lint, format, DESIGN.md lint, token drift, unit tests
mise run instance-verify     # Build the app image and check a disposable Compose instance (Docker required; not part of the gate)
mise run external-db-verify  # Check external TLS startup and retained state after recreation (Docker required; not part of the gate)
```

`mise run check` runs its four parts one at a time, in the order above, and stops
at the first one that fails. Without mise, run the same four commands by hand: see
step 6 of [Without mise](#without-mise). Step 7 lists the command behind each of the
other tasks.

When you add or change a task, keep a body of up to about six lines inline in
`mise.toml`. Put a longer body in a POSIX `sh` script under `scripts/` or
`server/scripts/`, and make the task run that script, as `check-compose` does.
A task body must run without mise, so it never calls `mise` itself. When you
change a development task's body, change its row in step 7 of
[Without mise](#without-mise) to match.

`mise run check-compose` compares the resolved app services, checks the external entry's
services and volumes, and verifies URL requirements and mount settings. It needs
Docker Compose and `jq`, but no running Docker daemon. CI runs it on every pull request.

`mise run external-db-verify` builds the app and uses one disposable PostgreSQL fixture
in a separate Compose project. It checks an absent certificate directory is refused,
starts through the read-only CA mount against a TLS-only database, then recreates
the app and compares migration rows, pairing, the storage marker, and a stored file.
The fixture container and its network must survive app removal.

Server safeguards run in the Rust gate: [startup tests](server/crates/core/src/database/startup/tests.rs)
cover unrelated data, incompatible migration history, and TLS rejection without writes;
[configuration tests](server/crates/server/src/config.rs) cover transport policy and certificate-file
refusals; [server database tests](server/crates/server/tests/database.rs) cover safe startup
errors and trusted TLS; [storage tests](server/crates/server/tests/storage.rs) cover pairing
and missing markers; and [readiness tests](server/crates/server/tests/readiness/mod.rs)
cover database outage and recovery. Keep these cases there when changing deployment checks.

## 🧰 Development Environment

The repo pins its tools — Rust, Node, pnpm, `gh`, the `openspec` CLI and
[Let's Spec](https://github.com/sidereal-io/lets-spec) — through mise. mise is
not required: you can install the same tools by hand. Pick one of the two
routes below, [With mise](#with-mise) or [Without mise](#without-mise).

If you cloned before the Rust workspace moved from `backend/` to `server/`, run
`rm -rf backend/target` once after you pull. Git ignores that folder, so it stays behind.

### With mise

1. Install [mise](https://mise.jdx.dev/getting-started.html). Use a recent
   release: an old mise may fail to read `mise.lock`.
2. Add `mise activate` to your shell's startup file, as mise's
   [activation guide](https://mise.jdx.dev/getting-started.html#activate-mise)
   shows. For bash, add `eval "$(mise activate bash)"` to `~/.bashrc`.
3. In the repo root, trust the repo's `mise.toml` and install its tools, as one
   command:

   ```
   mise trust && mise install
   ```

   Run them together. Once the repo is trusted, your next prompt runs the
   `enter` hook in `mise.toml`, and its `lets sync` fails until `mise install`
   has installed `lets`.

   `mise.toml` names each tool, and `mise.lock` pins each tool's exact release
   and checksum. mise reads the Node version from `.nvmrc`.

   `mise.lock` must pin each tool for Linux x64, Linux arm64 and macOS arm64.
   Rust is the exception: it records a channel, not a download. The `mise`
   workflow fails when a tool lacks one of these platforms, and names the tool
   and the platform. To run the same check yourself, run
   `scripts/check-mise-lock-platforms.sh` in the repo root.
4. mise installs Rust through rustup, on the `stable` channel. `mise.lock`
   records the channel, not a release, so rustup's newest stable release
   decides it. Run `rustup update` to move to a newer release.
5. Nothing to run for the agent skills. Entering the repo directory runs
   `lets sync`, which keeps them at the pinned Let's Spec release. See
   [Agent skills](#agent-skills).

Keep personal mise settings in `mise.local.toml`, next to `mise.toml`. Git
ignores it, and mise applies it over `mise.toml`, so your settings take
precedence.

**Run `mise trust --untrust` before you check out a branch you don't trust.**
Entering the repo runs the `enter` hook in the checked-out `mise.toml`. mise
trusts that file by its path, not its contents, so a checkout can run a
branch's hook without asking. After you read the branch's changes, run
`mise trust` again.

### Without mise

1. Install [rustup](https://rustup.rs/). It reads `server/rust-toolchain.toml`,
   which names the `stable` channel, and uses the latest stable Rust release you
   have installed. Run `rustup update` to move to a newer release.

   The `ci` workflow installs Rust's `stable` channel through mise when each
   job runs, so it uses the latest stable release, as you do after
   `rustup update`.
2. Install a Node version manager (nvm, fnm, or similar) that reads `.nvmrc`,
   and run its "use" command in the repo root to select Node 26.
3. Install pnpm 12 once. The web app in `web/` uses it:

   ```
   npm i -g pnpm@12
   ```

   In `web/`, pnpm switches itself to the exact release that
   `web/package.json` pins, so any pnpm 12 works. See
   [`web/README.md`](web/README.md).
4. Install the [GitHub CLI](https://cli.github.com/), `gh`, 2.102.0 or later. The agent
   skills set Project fields and issue links with it.
5. Install the `openspec` CLI at OpenSpec 2.1.1. Download the file for your
   system from its
   [GitHub release](https://github.com/Fission-AI/OpenSpec/releases/tag/v2.1.1),
   and put the `openspec` binary it holds on your `PATH`. Or install it with
   npm:

   ```
   npm install -g @fission-ai/openspec@2.1.1
   ```

   The agent skills, schemas and rules are committed, so you have them without
   running anything. Only mise keeps them up to date: see
   [Agent skills](#agent-skills).
6. Run the gate. These are the four commands that `mise run check` runs. Run
   them from the repo root, in this order, and stop at the first one that
   fails. The server checks need the PostgreSQL fixture and
   `TEST_DATABASE_URL`, as the next paragraph says.

   1. Compose configuration checks. They need the Compose CLI and `jq`, but no
      running Docker daemon:

      ```
      sh scripts/compose-config-check.sh
      ```
   2. OpenSpec checks: strict validation of every spec and active change. A
      warning fails them:

      ```
      OPENSPEC_TELEMETRY=0 openspec validate --all --strict --no-interactive
      ```
   3. Server checks: format, clippy with warnings denied, tests, and the
      dependency-direction lint:

      ```
      (cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test) && server/scripts/check-arch.sh
      ```
   4. Web checks: install from the lockfile, then types, lint, format,
      `DESIGN.md` lint, token drift and unit tests:

      ```
      (cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck && pnpm lint && pnpm format:check && pnpm design:lint && pnpm tokens:check && pnpm test)
      ```

   The parentheses keep each `cd` inside its own command, so you stay in the
   repo root for the next one.
7. Run a development task. Each task in `mise.toml` runs one command, and runs
   the same way without mise. Run it from the repo root:

   | Task | Command |
   |---|---|
   | `db-up` | `sh server/scripts/db-fixture.sh up` |
   | `db-down` | `sh server/scripts/db-fixture.sh down` |
   | `db-clean` | `sh server/scripts/db-fixture.sh clean` |
   | `db-url` | `sh server/scripts/db-fixture.sh url` |
   | `db-test-url` | `sh server/scripts/db-fixture.sh test-url` |
   | `db-test-clean` | `sh server/scripts/db-fixture.sh orphan-clean` |
   | `db-tls` | `sh server/scripts/db-tls.sh` |
   | `db-schema` | `sh server/scripts/db-schema.sh` |
   | `server` | `sh scripts/dev.sh server` |
   | `web` | `sh scripts/dev.sh web` |
   | `dev` | `sh scripts/dev.sh` |
   | `e2e` | `sh scripts/e2e.sh` |
   | `instance-verify` | `sh scripts/instance-verify.sh` |
   | `external-db-verify` | `sh scripts/external-db-verify.sh` |

   For example, prepare the fixture and run the stack:

   ```
   sh server/scripts/db-fixture.sh up
   export DATABASE_URL=$(sh server/scripts/db-fixture.sh url)
   sh scripts/dev.sh
   ```

Every route passes the same gate: `mise run check`, or its four commands by hand.
Before you run it, prepare the PostgreSQL fixture and export `DATABASE_URL` and
`TEST_DATABASE_URL` using the commands under Development Commands.

On both routes, some tasks need tools that mise does not install. Install them
from your system's package manager:

- **The fixture:** Docker Engine or Docker Desktop, with Compose v2 or newer,
  and `openssl` on `PATH`. The first `db-up` in a checkout makes the fixture's
  certificates with `openssl`, before it starts Compose. Without `openssl`, it
  stops with no error message. `db-tls` and `external-db-verify` need
  `openssl` too.
- **`dev`,** which runs the server and the web shell together: `setsid`, or
  `perl` where there is no `setsid` command. Most Linux systems have `setsid`,
  from util-linux, and macOS has `perl`. Without either, `dev` stops with an
  error that names them. `server` and `web` on their own need neither.

See [server setup](server/README.md) for container-free test prerequisites.

### Moving from the Nix shell

The repo no longer has a Nix shell or an `.envrc`. If you used them, do this
once, in the repo root:

1. If you opened the shell with `nix develop`, without direnv, leave it first:
   run `exit`, or open a new terminal. Deleting `.direnv/` does not end that
   shell, and its tool and library paths would stay ahead of mise's.
2. Delete `.direnv/`. direnv unloads the old shell by itself once `.envrc` is
   gone. To clear direnv's record of approved folders too, run `direnv prune`;
   this is optional.
3. If you kept settings in `.envrc.local`, move them to `mise.local.toml`, as
   TOML under an `[env]` table. For example, `export RUST_LOG=debug` becomes
   `RUST_LOG = "debug"`. A line that put a folder first on `PATH`, such as
   `export PATH="$PWD/.local-bin:$PATH"`, becomes mise's
   [`_.path`](https://mise.jdx.dev/environments/) setting:

   ```toml
   [env]
   RUST_LOG = "debug"
   _.path = ".local-bin"
   ```

   mise reads a relative path from the folder that holds `mise.local.toml`,
   and puts it ahead of the tools it installs. To check, run
   `mise env --paths`: the folder is on the first line.

   Git still ignores `.envrc.local`, so you can delete it when you are done.
4. Follow [With mise](#with-mise). mise is also in Nixpkgs, so you can install
   it with Nix if you prefer.

### Where versions are set

Change a version only where the table below says it is set.

**Tool versions** decide which tools you build and check with.

| Tool | Set in | How exact |
|---|---|---|
| PostgreSQL | `server/postgres-image.env` | Exact PostgreSQL 18 patch and image digest, shared by development and CI. `compose.yml` holds a copy for the Compose instance. Change both together; `mise run instance-verify` fails when they differ |
| Rust | `server/rust-toolchain.toml`, and `mise.toml` with `mise.lock` | Both files name the `stable` channel, and the `mise` workflow fails when they differ. In the `ci` workflow and with mise, `mise.lock` records the channel and rustup decides the release. With your own tools, rustup uses the latest stable release you have installed |
| Node | `.nvmrc` | Major version 26. mise reads `.nvmrc`, and `mise.lock` pins the exact release. The root `Dockerfile` names the same major version for the image's web build |
| pnpm | `packageManager` in `web/package.json` | One exact release. Any pnpm 12 switches itself to it in `web/`. `mise.toml` names pnpm 12, and `mise.lock` pins the release |
| Let's Spec, with `gh` | `mise.toml` with `mise.lock` | One release of [Let's Spec](https://github.com/sidereal-io/lets-spec), named in `mise.toml`. `mise.toml` names `gh` 2, the CLI the skills call, and `mise.lock` pins both |
| `openspec` | `mise.toml` with `mise.lock` | OpenSpec 2.1.1, the release binary from [GitHub](https://github.com/Fission-AI/OpenSpec/releases/tag/v2.1.1). `mise.toml` names the release, and `mise.lock` pins each download's checksum. Pinned on its own until a Let's Spec release names OpenSpec 2: don't take its version from the Let's Spec release notes. With your own tools, install 2.1.1 |

Never write a Rust release number in a build file or a document. The `stable` channel decides it, and rustup picks the release.

Dependabot proposes updates to these pins every week: `server/Cargo.lock` and the packages in `web/`. Dependabot does not read `mise.toml`. Instead, the weekly `mise-tool-updates.yml` workflow opens one pull request that moves Node, pnpm and `gh` within their major versions, and Let's Spec to its newest release, in `mise.toml` and `mise.lock`. It leaves OpenSpec and Rust alone. The pull request changes only `mise.toml`, `mise.lock` and the files `lets sync` writes: `scripts/apply-mise-tool-update.sh` refuses any other file before the workflow opens it. See [Dependency updates](docs/dependency-updates.md#weekly-mise-tool-updates).

**Product versions** are Sidereal's own version numbers.

- `version` under `[workspace.package]` in `server/Cargo.toml` sets every crate's version. It is `0.1.0` today.
- `version` in `web/package.json` is the web interface's. It is `0.0.0` today.
- `main` has not released yet. Its first release is planned as `v2.0.0`. The change that defines how `main` releases will align these two numbers.
- Released versions are git tags. The v0.10.x releases are tagged from the `v0.x` branch.

### Agent skills

AI agents read skills from `.agents/skills`. Claude Code reads the same folder
through the `.claude/skills` link. The folder holds two kinds of skill:

- **Authored skills**, such as `visual-options`. People write them, and git
  tracks them.
- **Let's Spec skills**, named `lets-*` and `openspec-*`. `lets sync` copies
  them from the pinned [Let's Spec](https://github.com/sidereal-io/lets-spec)
  release, with the `lets-*` schemas in `openspec/schemas/` and the rules block
  in `AGENTS.md`. Git tracks the copies, so every clone has them.

mise runs `lets sync` when you enter the repo directory. It rewrites any copy that differs
from the release and tells you to commit. When all is well, it prints only one
warning: the OpenSpec CLI on `PATH` is 2.1.1, but the release's `openspec-*`
skills were generated from an older OpenSpec. Expect that warning on every
entry until a Let's Spec release names OpenSpec 2, and don't pin the CLI back
as it suggests. CI runs `lets sync --check`, which fails when a committed copy is out of
date. Never edit a `lets-*` or `openspec-*` skill, a `lets-*` schema or the
rules block by hand: the next sync undoes it. Change Let's Spec instead.

The weekly `mise-tool-updates.yml` pull request moves Let's Spec for you: it
changes `mise.toml` and `mise.lock`, and commits what `lets sync` wrote. Read
the release's upgrading notes before you merge it.

To move to a newer Let's Spec by hand:

1. Read its
   [upgrading notes](https://github.com/sidereal-io/lets-spec/blob/main/docs/upgrading.md).
   Some releases need a step of your own.
2. In `mise.toml`, change the `github:sidereal-io/lets-spec` version to the
   new release. Leave the `github:Fission-AI/OpenSpec` version alone: OpenSpec
   is pinned on its own until a Let's Spec release names OpenSpec 2.
3. Run `mise lock`, then `mise exec -- lets sync`.
4. Commit what `lets sync` wrote with `mise.toml` and `mise.lock`, in one pull
   request.

Don't run `openspec init` or `openspec update` by hand. They write skills of
their own from your global settings. If you do, run `lets sync` to restore the
repo's skills.

To support another agent, pick the case that fits it:

- **It reads `.agents/skills`:** do nothing.
- **It reads only its own folder:** add a tracked link from that folder to
  `.agents/skills`, as `.claude/skills` does.

## 📝 Code Standards

### Rust
- `cargo fmt` formats the code, and `cargo clippy` runs with warnings denied
- Keep the dependency direction in [`AGENTS.md`](AGENTS.md#durable-constraints-server): packs depend on `plugin-abi`, never on `core`
- Keep astronomy logic in `packs/astro`, never in `core`

### TypeScript
- Use strict TypeScript settings
- Avoid `any` types
- Follow the ESLint and Prettier configurations in `web/`
- Use design tokens from [`DESIGN.md`](DESIGN.md), never raw values

### Commit Messages
Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add plate solving progress tracking
fix: resolve thumbnail loading issues
docs: update deployment instructions
refactor: improve database connection handling
test: add unit tests for image processing
```

### File Organization
- The server is a cargo workspace in `server/`. Its crates live in `server/crates/`
- The web interface lives in `web/`, with its source in `web/src/`

## 🧪 Testing

### Running Tests
```bash
# Prepare the database prerequisite, then run every check
mise run db-up
export DATABASE_URL=$(mise run db-url)
export TEST_DATABASE_URL=$(mise run db-test-url)
mise run check

# Run the server tests only
cd server && cargo test

# Run the web tests only
cd web && pnpm test
```

### Writing Tests
- Write tests for new features
- Include edge cases and error conditions
- Test both success and failure scenarios
- Mock external dependencies (APIs, databases)

## 📦 Pull Request Process

### Before Submitting
1. **Test** your changes thoroughly
2. **Run** the checks:
   ```bash
   mise run check
   ```
3. **Update** documentation if needed
4. **Add** tests for new functionality

### Pull Request Guidelines
- Use a clear, descriptive title
- Reference related issues with `Fixes #123`
- Provide a detailed description of changes
- Include screenshots for UI changes
- Keep PRs focused and atomic

### PR Template
```markdown
## Summary
Brief description of the changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed

## Screenshots (if applicable)
[Add screenshots here]

## Checklist
- [ ] Code follows project standards
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added/updated
```

## 🏗️ Architecture Guidelines

- Read [`docs/architecture.md`](docs/architecture.md) for the target design, and
  [`docs/decisions/`](docs/decisions/) for the decisions behind it.
- Follow the durable constraints in [`AGENTS.md`](AGENTS.md). They apply to people
  as much as to agents.
- The server uses PostgreSQL only.

## 🔒 Security Guidelines

- Never commit secrets or API keys
- Use environment variables for configuration
- Validate all user inputs
- Implement proper authentication
- Follow OWASP security guidelines

## 📚 Resources

### Documentation
- [Project README](README.md)
- [Server README](server/README.md)
- [Web README](web/README.md)
- [Design system](DESIGN.md)

### External Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)

## 🤝 Community

### Code of Conduct
- Be respectful and inclusive
- Welcome newcomers
- Help others learn
- Keep discussions constructive

### Getting Help
- Join discussions in GitHub Issues
- Ask questions in pull requests
- Check existing documentation first
- Provide context when asking for help

### Communication
- Use clear, descriptive language
- Be patient with responses
- Provide helpful feedback
- Acknowledge contributions

## 🎯 Development Priorities

### High Priority
- Bug fixes affecting core functionality
- Security improvements
- Performance optimizations
- Documentation improvements

### Medium Priority
- New features with clear use cases
- UI/UX improvements
- Integration enhancements
- Testing improvements

### Low Priority
- Nice-to-have features
- Code refactoring
- Experimental features
- Tool improvements

## 📄 License

By contributing to Sidereal, you agree that your contributions will be licensed under the MIT License.

## 🙏 Recognition

Contributors are recognized in:
- GitHub contributors list
- Release notes for significant contributions
- README acknowledgments
- Community shout-outs

Thank you for contributing to Sidereal! Your help makes this project better for the entire astrophotography community.
