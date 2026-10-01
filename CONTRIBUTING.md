# Contributing to Sidereal

Thank you for your interest in contributing to Sidereal! We welcome contributions from the community to help make this astrophotography management tool even better.

## 🚀 Quick Start

1. **Fork** the repository on GitHub
2. **Clone** your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/Sidereal.git
   cd Sidereal
   ```
3. **Install** dependencies:
   ```bash
   npm install
   ```
4. **Create** a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```
5. **Make** your changes
6. **Test** your changes
7. **Submit** a pull request

## 📋 Types of Contributions

We welcome several types of contributions:

### 🐛 Bug Reports
- Use the GitHub issue tracker
- Include detailed reproduction steps
- Provide environment information (OS, Node.js version, etc.)
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

## 🛠️ Development Setup

### Prerequisites
- Node.js 26
- npm 10+
- Git
- Docker (optional, for database)

### Environment Setup
```bash
# Clone and install
git clone https://github.com/YOUR_USERNAME/Sidereal.git
cd Sidereal
npm install

# Setup environment
cp .env.example .env.local
# Edit .env.local with your development settings

# Initialize database
npm run db:generate
npm run db:migrate

# Start development server
npm run dev
```

### Development Commands
```bash
# Development
npm run dev            # Start backend server
npm run dev:watch      # Start with file watching
npm run dev:worker     # Start worker process
npm run dev:all        # Start backend + worker

# Building
npm run build          # Build for production
npm run check          # TypeScript type checking

# Database
npm run db:generate    # Generate migrations
npm run db:migrate     # Apply migrations
npm run db:studio      # Open database GUI

# Code Quality
npm run lint           # ESLint checking
npm run format         # Prettier formatting
npm run test           # Run tests
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
3. If you used a different major version of Node before, run `npm rebuild` once.
   `better-sqlite3` is a native module built for one Node version, and Nix's
   Node 26 needs its own build.
4. Expect a full rebuild of `server/target` the first time you run a Rust
   command in the shell, and again whenever `flake.lock` brings a new Rust
   release. The shell's compiler is not the one rustup installed, so cargo's
   cached build data doesn't carry over.
5. Nothing to run for the OpenSpec agent skills. The shell generates them when
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

Both routes pass the same `just check` gate.

**Run `direnv deny` before you check out a branch you don't trust.** Loading
the shell runs code from the working tree: the flake, the `justfile` and
`scripts/skills.sh`. direnv asks for approval again only when `.envrc` changes,
so a checkout can run a branch's code without asking. After you read the
branch's changes, run `direnv allow` again.

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

### Reviewing a pin update

Every Monday, Dependabot may open pull requests that update the pinned tools.
Check these points before you merge one:

- **Nix pull requests change `flake.lock`.** The flake check CI job builds the
  shell with the new pins. A green check means the shell builds. The job log
  also prints the Rust, `openspec`, Node, pnpm, and `just` versions inside the new
  shell.
  The `v2` workflow's `server` job also runs, and it runs `just check-server`
  inside the new shell. A green check means the server gate passes with the
  new pins.
- **A `flake.lock` update can bring a new Rust release.** The
  `rustc --version` line in the flake check log shows the release the update
  brings. A new release can add clippy lints that fail the `server` job. If it does, fix the
  lints in a separate pull request to `main`, then comment
  `@dependabot rebase` on the update.
- **A new `openspec` version changes every agent's skills.** The shell
  regenerates the skills from the CLI, so read the
  [`openspec` release notes](https://github.com/Fission-AI/OpenSpec/releases)
  before you merge an update that changes its version.
- **Cargo pull requests change `server/Cargo.lock`.** The `v2` workflow's
  `server` job checks them, the same as any other change under `server/`.

## 📝 Code Standards

### TypeScript
- Use strict TypeScript settings
- Provide proper type definitions
- Avoid `any` types when possible
- Use Zod schemas for validation

### Code Style
- Follow ESLint and Prettier configurations
- Use meaningful variable and function names
- Write clear, concise comments
- Keep functions small and focused

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
- Place components in `apps/client/src/components/`
- API routes go in `apps/server/src/routes/`
- Shared types in `packages/shared/src/types/`
- Database schemas in `packages/shared/src/db/`

## 🧪 Testing

### Running Tests
```bash
# Run all tests
npm run test

# Run tests in watch mode
npm run test:watch

# Run specific test file
npm run test -- --testNamePattern="image processing"
```

### Writing Tests
- Write tests for new features
- Include edge cases and error conditions
- Test both success and failure scenarios
- Mock external dependencies (APIs, databases)

## 📦 Pull Request Process

### Before Submitting
1. **Test** your changes thoroughly
2. **Run** code quality checks:
   ```bash
   npm run lint
   npm run format
   npm run check
   npm run test
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

### Database Changes
- Use Drizzle ORM for database operations
- Create migrations for schema changes
- Test migrations on sample data
- Consider backward compatibility

### API Design
- Follow RESTful conventions
- Use proper HTTP status codes
- Implement proper error handling
- Include request/response validation

### Frontend Components
- Use shadcn/ui components when possible
- Follow React best practices
- Implement proper error boundaries
- Use TypeScript for all components

### Worker Processes
- Handle errors gracefully
- Implement proper logging
- Use queues for background tasks
- Consider resource limitations

## 🔒 Security Guidelines

- Never commit secrets or API keys
- Use environment variables for configuration
- Validate all user inputs
- Implement proper authentication
- Follow OWASP security guidelines

## 📚 Resources

### Documentation
- [Project README](README.md)
- [Docker Documentation](docker/README.md)
- [API Documentation](docs/api.md)

### External Resources
- [Node.js Best Practices](https://nodejs.dev/en/learn/)
- [React Documentation](https://react.dev/)
- [TypeScript Handbook](https://www.typescriptlang.org/docs/)
- [Drizzle ORM Docs](https://orm.drizzle.team/)

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