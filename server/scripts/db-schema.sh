#!/bin/sh
# Regenerate the normalized schema descriptions against the pinned fixture.
# Run through `mise run db-schema`, or as `sh server/scripts/db-schema.sh`.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd -P)
cd "$root"
sh server/scripts/db-fixture.sh up
TEST_DATABASE_URL=$(sh server/scripts/db-fixture.sh test-url)
export TEST_DATABASE_URL
SIDEREAL_GENERATE_SCHEMA=1 cargo test --manifest-path server/Cargo.toml -p sidereal-core schema_baselines_are_generated_and_stable
