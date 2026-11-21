#!/bin/sh
# End-to-end tests: boot the server and Vite, then drive Chromium against
# them. Run through `mise run e2e`, or as `sh scripts/e2e.sh` without mise.
# The database fixture must already be up (`mise run db-up`); the webServer
# blocks in web/playwright.config.ts start everything else. Reuses an
# already-running `mise run dev`, so locally this only pays for the browser
# launch.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

# The server reads DATABASE_URL, not TEST_DATABASE_URL, and exits without
# it. Fall back to the local fixture when the caller has not picked one;
# server/scripts/db-schema.sh resolves its own URL the same way.
if [ -z "${DATABASE_URL:-}" ]; then
  DATABASE_URL=$(sh server/scripts/db-fixture.sh url)
  export DATABASE_URL
fi
cd web
pnpm install --frozen-lockfile --reporter=append-only
# Playwright owns its own browser. The download is skipped when the browser
# this version wants is already in the cache, so a bare `mise run e2e`
# keeps working with no setup step.
pnpm exec playwright install chromium
pnpm e2e
