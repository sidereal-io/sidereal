# 002: Remove npm from the final runtime image

**Status:** Accepted
**Date:** 2026-10-06
**Context:** Issue #309 release validation; v0.10.4 failed its container security gate.

## Problem

The final Node 24 image contains npm tooling whose bundled brace-expansion and undici dependencies have three fixable HIGH findings. Both npm 11.21.0 and the actual npm 12.2.0 distribution retain the vulnerable versions. The application startup, database migrations, server, and worker execute Node directly; npm is required while installing dependencies and building, but those runtime paths do not use it.

Should the final image retain package-management tooling, patch npm's private dependencies, or remove that tooling after the production dependency install?

## Options

### Option A: Remove npm and npx after production installation
**Pros:**
- Removes the vulnerable tooling from the shipped runtime rather than managing npm's internal dependency tree.
- Keeps npm available in the builder and until runtime production dependencies finish installing.
- Leaves Node and the application's dependencies intact.

**Cons:**
- Operators can no longer run `docker exec <container> npm ...` or `npx ...`; debugging or package-management tasks require a development image or a separate Node container.
- Future runtime scripts must continue to invoke Node directly.

Concrete final-stage cleanup, after `npm ci --omit=dev` and cache cleanup:

```dockerfile
RUN rm -rf /usr/local/lib/node_modules/npm /usr/local/bin/npm /usr/local/bin/npx
```

Do not remove Node or Corepack if the base provides it.

### Option B: Surgically update npm's bundled dependencies
**Pros:**
- Preserves npm/npx in the final container.

**Cons:**
- Mutates npm's published internal dependency tree and requires compatibility checks for npm itself.
- Introduces maintenance for nested copies, dependency requirements, and upstream layout changes.
- Leaves package installation capability in the application runtime.

### Option C: Wait for a patched upstream npm distribution
**Pros:**
- Preserves upstream-supported npm contents and operator workflows.

**Cons:**
- Blocks the authorized application release with no confirmed upstream patch availability.
- The evaluated npm 12.2.0 distribution still contains brace-expansion 5.0.9 and undici 6.28.0.

## Recommendation

Choose Option A. The shipped application uses Node directly, so removing package-manager tooling after installation is the smallest maintainable runtime change. Validate a real image with the existing smoke test and the unchanged release Trivy HIGH/CRITICAL gate, assert Node works and npm/npx are absent, and disclose the operator-facing tooling change in the changelog. Preserve the existing immutable failed v0.10.4 tag and publish a new patch release only after verification.

## Decision

The developer accepted Option A on 2026-10-06 after reviewing the operator impact and prototype validation. Remove npm/npx from the final runtime after production dependency installation; preserve Node, Corepack, and application dependencies. Publish the verified repair as v0.10.5 without moving the failed v0.10.4 tag.
