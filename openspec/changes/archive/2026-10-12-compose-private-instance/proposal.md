## Why

Sam has no way to install Sidereal from `main`. The server image holds only the health server, with no web interface, no database service, and no Compose file.
The PostgreSQL foundation is merged, so one Compose file can now start the app and its database together.

## What Changes

- Add a root `compose.yml` that starts the app and PostgreSQL 18 with one command.
- Add `.env.example`, which documents every setting and holds no password.
- Require `POSTGRES_PASSWORD` in `.env`. Compose refuses to start without it.
- Build one app image from the repo root. It holds the server binary and the built web shell.
- Serve the built web shell from the server when `WEB_ROOT` names its folder. The browser reaches the UI and the health probes on one origin.
- Keep the database in a named volume, so migration state survives container recreation.
- Mount a named storage volume that the app user can write. No server code uses it yet.
- Publish the app on host port 5000. Publish no database port.
- Add the root recipe `just instance-verify`. It builds the image, starts a disposable instance, checks it, and removes it.
- Add an installation guide for Sam.
- **BREAKING**: `server/Dockerfile` moves to a root `Dockerfile`. `docker build server/` no longer builds the image.

## Capabilities

### New Capabilities

- `compose-instance`: The Compose installation. It covers the password setting, the one-origin app image, the network surface, the retained volumes, and the installation guide.

### Modified Capabilities

- `dev-commands`: Adds the `just instance-verify` recipe, which stays outside `just check`.
- `dev-environment`: The Rust image rule points at the root `Dockerfile`. The Node rule also covers the image's web build stage.

## Impact

- **Server**: the `server` crate gains static file serving and the optional `WEB_ROOT` setting. It adds the `tower-http` dependency. `core`, `plugin-abi`, and `packs/astro` do not change.
- **Web**: no source change. The image runs the existing `pnpm build`.
- **Build files**: new root `Dockerfile`, `.dockerignore`, `compose.yml`, and `.env.example`. `server/Dockerfile` and `server/.dockerignore` are removed.
- **Recipes and scripts**: one new root recipe and its script.
- **Docs**: a new installation guide, with updates to `README.md`, `server/README.md`, and the version table in `CONTRIBUTING.md`.
- **CI**: no change. The `ci` spec forbids a workflow on `main` from building an image, so verification stays a local recipe.
- **Database**: no change. The app reuses the foundation's configuration, migrations, schema checks, and readiness.

## Non-goals

- **Authentication.** Until the authentication epic lands, the app serves only static web files and the two health probes. It exposes no library data and no mutation.
- **Storage behavior.** A later story owns storage settings, bind mounts, and permission checks. This change only mounts a writable volume.
- **Diagnostics and recovery.** A later story owns operator guidance for failed starts, including a changed password.
- **An existing PostgreSQL service.** A later story owns connecting the app to a database that Compose does not run.
- **Image publishing.** Sam builds the image locally. This change defines no release process for `main`.
- **A setup wizard and library operations.**
