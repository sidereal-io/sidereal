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
5. **Make** your changes, and run `just check`
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

The root `justfile` runs everything. `just --list` describes every recipe.

```bash
just db-up          # Start the isolated PostgreSQL fixture (Docker + Compose required)
export DATABASE_URL=$(just db-url)
export TEST_DATABASE_URL=$(just db-test-url)
just dev            # Run the server and the web interface together
just server         # Run the server only
just web            # Run the web interface only
just check          # The gate to pass before every pull request
just check-server   # Server checks: format, clippy, tests, dependency-direction lint
just check-web      # Web checks: types, lint, format, DESIGN.md lint, token drift, unit tests
```

## 🧰 Development Environment

The repo pins its tools — Rust, Node, pnpm, `just`, and the `openspec` CLI — through a
Nix flake. Nix is optional. Pick either route.

If you cloned before the Rust workspace moved from `backend/` to `server/`, run
`rm -rf backend/target` once after you pull. Git ignores that folder, so it stays behind.

### With Nix

1. Install [Nix](https://nixos.org/download/) with flakes enabled.
   [Determinate's installer](https://determinate.systems/nix-installer/) enables
   flakes by default. The plain installer at nixos.org does not: after it, add
   this line to `/etc/nix/nix.conf` (multi-user install) or
   `~/.config/nix/nix.conf` (single-user install), then restart the Nix daemon
   if you have one:

   ```
   experimental-features = nix-command flakes
   ```
2. Install [direnv](https://direnv.net/) and run `direnv allow` in the repo root.
   Entering the repo directory now loads the pinned shell automatically. Leaving
   restores your previous environment.

   Without direnv, run `nix develop` by hand instead. It gives the same shell for
   that one terminal session.
3. Expect a full rebuild of `server/target` the first time you run a Rust
   command in the shell, and again whenever `flake.lock` brings a new Rust
   release. The shell's compiler is not the one rustup installed, so cargo's
   cached build data doesn't carry over.
4. Nothing to run for the OpenSpec agent skills. The shell generates them when
   it loads, and again only when they are stale. See
   [Agent skills](#agent-skills).

Nix sees only files tracked by git. A new file under `nix/` stays invisible to
the shell until you run `git add` on it.

Keep personal direnv settings in `.envrc.local`, next to `.envrc`. Git ignores
it, and it loads after the pinned shell, so your settings take precedence.

### Without Nix

1. Install [rustup](https://rustup.rs/). It reads `server/rust-toolchain.toml`,
   which names the `stable` channel, and uses the latest stable Rust release you
   have installed. Run `rustup update` to move to a newer release.

   CI can be up to about two weeks behind the latest stable release, because it
   takes Rust from `flake.lock`. To see CI's release, open the "Tool versions"
   step in the log of the most recent `nix` flake check run.
2. Install a Node version manager (nvm, fnm, or similar) that reads `.nvmrc`,
   and run its "use" command in the repo root to select Node 26.
3. Install pnpm 12 once. The web app in `web/` uses it:

   ```
   npm i -g pnpm@12
   ```

   In `web/`, pnpm switches itself to the exact release that
   `web/package.json` pins, so any pnpm 12 works. See
   [`web/README.md`](web/README.md).
4. Install [`just`](https://github.com/casey/just).
5. Install the `openspec` CLI, then run `just skills`:

   ```
   npm install -g @fission-ai/openspec
   just skills
   ```

   The Nix shell pins the CLI version that the repo tests with. Without Nix,
   you get the latest release, so your skill text can differ slightly.
6. Optional: refresh the skills each time you enter the repo, as the Nix shell
   does. Install [direnv](https://direnv.net/), add this line to `.envrc.local`
   in the repo root, then run `direnv allow`:

   ```
   just enter
   ```

   The `enter` recipe regenerates the skills only when they are stale. If it
   fails, it prints one warning line, and direnv still loads.

Both routes pass the same `just check` gate. Prepare the PostgreSQL fixture and export
`DATABASE_URL` and `TEST_DATABASE_URL` using the commands above. Docker Engine or
Docker Desktop with Compose v2 or newer runs the fixture; Python 3 runs `just db-demo`.
See [server setup](server/README.md) for container-free test prerequisites.

**Run `direnv deny` before you check out a branch you don't trust.** Loading
the shell runs code from the working tree: the flake, the `justfile` and
`scripts/skills.sh`. direnv asks for approval again only when `.envrc` changes,
so a checkout can run a branch's code without asking. After you read the
branch's changes, run `direnv allow` again.

### Where versions are set

Each version lives in one file. Change it there, and nowhere else.

**Tool versions** decide which tools you build and check with.

| Tool | Set in | How exact |
|---|---|---|
| Python 3 | `flake.lock` | Pinned through Nixpkgs in the Nix shell; install Python 3 when using system tools |
| PostgreSQL fixture | `server/postgres-image.env` | Exact PostgreSQL 18 patch and image digest, shared by development and CI |
| Rust | `server/rust-toolchain.toml` | Names the `stable` channel. In the Nix shell and CI, `flake.lock` decides the exact release. Without Nix, rustup uses the latest stable release you have installed |
| Node | `.nvmrc` and `nix/toolchains.nix` | Major version 26 in both. Keep them in step |
| pnpm | `packageManager` in `web/package.json` | One exact release. Any pnpm 12 switches itself to it in `web/`. `nix/toolchains.nix` provides pnpm 12 |
| `just` and `openspec` | `flake.lock` | Exact releases, through the Nix shell |

Never write a Rust release number in a build file or a document. The channel and `flake.lock` decide it.

Dependabot proposes updates to these pins every week: `flake.lock`, `server/Cargo.lock`, and the packages in `web/`. See [Reviewing a pin update](#reviewing-a-pin-update).

**Product versions** are Sidereal's own version numbers.

- `version` under `[workspace.package]` in `server/Cargo.toml` sets every crate's version. It is `0.1.0` today.
- `version` in `web/package.json` is the web interface's. It is `0.0.0` today.
- `main` has not released yet. Its first release is planned as `v2.0.0`. The change that defines how `main` releases will align these two numbers.
- Released versions are git tags. The v0.10.x releases are tagged from the `v0.x` branch.

### Agent skills

AI agents read skills from `.agents/skills`. Claude Code reads the same folder
through the `.claude/skills` link. The folder holds two kinds of skill:

- **Authored skills**, such as `critique` and `discovery`. People write them, and
  git tracks them.
- **Generated skills**, named `openspec-*`. `just skills` creates them, and git
  ignores them.

The Nix shell refreshes the generated skills when it loads, but only when they
are stale. They are stale when any of these changed since the last run: the
`openspec` version, `.config/openspec/config.json` or `scripts/skills.sh`. A
missing generated folder also makes them stale. `just skills` records its
inputs in `.agents/skills/.openspec-stamp`, which git ignores.

`just skills` still forces a full run whenever you call it. For a given CLI
version, it gives the same skills on every machine:

- It stops if `openspec` is not installed. The error message names the install
  command.
- It deletes every `openspec-*` skill, the `.openspec-target` marker and the
  stamp, then generates them again.
- It reads the skill settings from the repo, never from your own
  `~/.config/openspec/config.json`. It leaves that file unchanged.

The skill settings live in `.config/openspec/config.json`: the profile, the
delivery mode and the workflow list. To add or remove a workflow, edit that
file and run `just skills`. The other OpenSpec file, `openspec/config.yaml`,
holds the schema, the project context and the artifact rules.

**The `openspec-` prefix is reserved.** `just skills` deletes any skill folder
with that prefix, so give an authored skill a different name.

Don't run `openspec init` or `openspec update` by hand. They use your global
settings instead of the repo's. If you do, run `just skills` to restore the
repo's skills.

To support another agent, pick the case that fits it:

- **It reads `.agents/skills`:** do nothing.
- **It reads only its own folder:** add a tracked link from that folder to
  `.agents/skills`, as `.claude/skills` does.
- **It needs different skill text:** it needs its own generated set. That takes
  a change to `scripts/skills.sh`.

### Penpot MCP

Agents design v2 screens in the **Sidereal Design System** file in Penpot. They
reach it through Penpot's MCP server. Each contributor connects it through
their own claude.ai account, so the repo holds no token and no MCP config.

You need a Penpot account and a claude.ai account, and you must sign in to
Claude Code with that claude.ai account. The connector doesn't work if you sign
in with an API key.

1. In Penpot, open **Your account**, then **Integrations**, then **MCP Server**,
   and turn on MCP.
2. Create an MCP key, then copy the server URL from the same page. Penpot shows
   the key only once.

   **The URL contains your key, so treat it like a password.** Don't paste it
   into the repo, an issue or a chat. Each Penpot user has one key. Creating a
   new key revokes the old one, so you then repeat step 3 with the new URL.
3. In claude.ai, open **Settings**, then **Connectors**, and add a custom
   connector named `Penpot` with the server URL.
4. Open the Sidereal Design System file in Penpot. Choose **File**, then
   **MCP Server**, then **Connect**.

   Keep that browser tab open while the agent works. The plugin runs inside the
   tab. If the browser puts the tab to sleep, the agent's Penpot calls fail
   until you connect again.
5. Start Claude Code and run `/mcp`. It lists **claude.ai Penpot** as connected.

If a step is missing, you see one of these:

- **`/mcp` doesn't list claude.ai Penpot:** you signed in to Claude Code with an
  API key or with another claude.ai account, or the connector isn't added.
- **The connector is listed, but Penpot calls fail:** the plugin isn't connected
  in the Sidereal Design System file, or its tab went to sleep. Repeat step 4.

If you set up Penpot by hand in Claude Code before, remove that setup in the
repo root. Otherwise the agent gets two copies of the Penpot tools:

```
claude mcp remove penpot -s local
```

### Reviewing a pin update

Every Monday, Dependabot may open pull requests that update the pinned tools.
Check these points before you merge one:

- **Nix pull requests change `flake.lock`.** The flake check CI job builds the
  shell with the new pins. A green check means the shell builds. The job log
  also prints the Rust, `openspec`, Node, pnpm, and `just` versions inside the new
  shell.
  The `ci` workflow's `server` and `web` jobs also run. They run
  `just check-server` and `just check-web` inside the new shell. Green checks
  mean both gates pass with the new pins.
- **A `flake.lock` update can bring a new Rust release.** The
  `rustc --version` line in the flake check log shows the release the update
  brings. A new release can add clippy lints that fail the `server` job. If it does, fix the
  lints in a separate pull request to `main`, then comment
  `@dependabot rebase` on the update.
- **A new `openspec` version changes every agent's skills.** The shell
  regenerates the skills from the CLI, so read the
  [`openspec` release notes](https://github.com/Fission-AI/OpenSpec/releases)
  before you merge an update that changes its version.
- **Cargo pull requests change `server/Cargo.lock`.** The `ci` workflow's
  `server` job checks them, the same as any other change under `server/`.
- **npm pull requests for `web/` change `web/package.json` and
  `web/pnpm-lock.yaml`.** The `ci` workflow's `web` job checks them, the same
  as any other change under `web/`.
- **Some pull requests target `v0.x`.** Dependabot sends version updates for
  the v0.10.x app's npm, Docker and GitHub Actions dependencies to the `v0.x`
  branch. Review them there, with `v0.x`'s own checks.

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
just db-up
export DATABASE_URL=$(just db-url)
export TEST_DATABASE_URL=$(just db-test-url)
just check

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
   just check
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