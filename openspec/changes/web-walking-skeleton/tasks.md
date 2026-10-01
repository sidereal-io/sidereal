## 1. Toolchain

- [x] 1.1 Add `pkgs.pnpm_12` to `nix/toolchains.nix`. Verify that `nix develop --command pnpm --version` prints a release starting with `12.`, and that `nix flake check` exits with status 0.
- [x] 1.2 Add `pnpm --version` to the flake check's "Tool versions" step in `.github/workflows/nix.yml`. Verify by running the step's command locally: it prints five versions and exits with status 0.

## 2. Web shell

- [x] 2.1 Scaffold `web/` from `create-vite`'s `react-ts` template. Remove the demo assets and styles, the ESLint config, and the ESLint packages (D1). Set `packageManager` to the newest pnpm 12 release, with no hash suffix (D2). Verify that `pnpm install` in `web/` writes `web/pnpm-lock.yaml` with `pnpm` under `packageManagerDependencies` at that release. Also verify that `grep -n '"web/' package.json package-lock.json` finds no match.
- [x] 2.2 Configure `web/vite.config.ts`: forward `/healthz` to `http://localhost:5000`, set `server.cors: false`, `server.strictPort: true`, and `clearScreen: false`, and keep the default host (D6, D7). Verify the scenarios of the `web-shell` requirements "reaches the server through its own origin", "allows no cross-origin reads", and "stays on this machine".
- [x] 2.3 Build the health-status component: `fetch` with a 5-second abort, the next check 5 seconds after each one finishes, three states, and no body text on screen (D5). Verify in a browser every scenario of the `web-shell` requirements "shows the server's health" and "keeps checking while it is open". Use a small local stub for the unexpected-body and no-answer scenarios.
- [x] 2.4 Verify the pnpm pin scenarios of the `web-shell` spec. With another pnpm 12 release on `PATH`, `pnpm --version` in `web/` prints the pinned release. With a changed `pnpm` hash in a scratch copy of the lockfile and an empty pnpm cache, pnpm refuses to run. Restore nothing in the repo: run the hash test in `.workspace/`.

## 3. Recipes

- [x] 3.1 Add the `web` recipe in group `v2`: `pnpm install --frozen-lockfile`, then `pnpm dev`, in `web/` (D4). Verify both scenarios of the `dev-commands` requirement "The web shell runs on its own".
- [x] 3.2 Replace the `dev` recipe with D3's POSIX `sh` recipe, in group `v2`. Verify every scenario of the `dev-commands` requirement "One command runs the v2 stack", including the fresh-clone scenario in a new clone under `.workspace/`.
- [x] 3.3 Rename `frontend` to `v0-frontend`, add `v0-dev` (`npm run dev`), and add the `v2` and `v0.10.x` groups (D8). Verify the scenarios of the `dev-commands` requirements "The v0.10.x stack runs by explicit name" and "Recipes are grouped by stack". If the v0.10.x stack cannot start on this machine, record why in the commit message.
- [x] 3.4 Update the `justfile`'s header comment so it names the v2 server, the v2 web shell, and the v0.10.x stack. Verify that `just --list` shows the new comment and every recipe under its group.
- [ ] 3.5 Test `just dev` on macOS with its `/bin/sh`: Ctrl-C stops both halves, and a taken port 5173 stops both. If no Mac is available, record that in the PR description.

## 4. Docs

- [x] 4.1 Write `web/README.md`: how to run the web shell, the network need on the first pnpm run, and that the web shell and the v0.10.x frontend share port 5173. Verify that every command in it runs as written.
- [x] 4.2 Update `CONTRIBUTING.md`: add pnpm to the Nix shell's tool list, and add the one-time `npm i -g pnpm@12` step for contributors without Nix. Verify that the `dev-environment` scenarios "The contributor docs name no Rust release" and "No outdated Node version is left in the docs" still pass.
- [x] 4.3 Update the recipe table in `server/README.md` and the "Run both stacks" line in `AGENTS.md`. Verify that `git grep -n 'just frontend'` finds no match outside `openspec/changes/archive/`.

## 5. Final check

- [ ] 5.1 Run `just check`, `nix flake check`, and `openspec validate web-walking-skeleton --strict`. Verify that each exits with status 0.
- [ ] 5.2 Walk every scenario in the three delta specs once more on the final commit, and record any that could not run, with the reason, in the PR description.
