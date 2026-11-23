## Why

Nova can upload an image, but Sidereal records nothing about it beyond what
upload itself stores. Every later processing story (format and dimensions,
previews, processing goals, plate solving) needs one working path from a
plugin to a stored, checked fact. This change builds that path with the
smallest fact there is, the file size, and gives Nova a page where the fact
shows.

## What Changes

- **The Operator contract becomes executable.** An Operator in `plugin-abi`
  declares the facet schemas its pack owns and the facets it writes, and runs
  through a run-scoped Asset context that lets it read the version's bytes,
  propose facet values, write log lines and check for cancellation (D4).
  **BREAKING:** implementers of the current pre-freeze `Operator` stub,
  including the astro pack's plate-solve stub, must implement the new methods.
  The ABI version goes up.
- **Core gains an executor, facet storage and operation runs.** Core runs a
  registered Operator over an Asset version, records one operation run for the
  attempt, and refuses any proposed facet that is unknown, invalid, or not in
  the Operator's declared facets. It stores each accepted value with its
  provenance: actor (the user who started the run), plugin and version, run,
  schema version and time (ADR-008). A pack compiled into the server has its
  grants from the build, fixed when the pack registers (D4). A finished run
  never changes, and each schema version is kept as values were written
  under it.
- **The image pack adds a file-size Operator.** It writes `image.file-size`,
  a whole number of bytes, on the Asset version. The value repeats
  `asset_versions.byte_size` on purpose (D3).
- **Upload runs the Operator.** Inside the same upload request, after the
  Asset is saved, the server runs the file-size Operator; the upload answers
  once the run has ended (D5). A client that goes away once the save can no
  longer be cancelled does not stop the save or the run. Only new uploads
  run it: images already in the
  library get no facts (D2). A failed or refused run is recorded as failed
  with its reason, writes no facet, and does not fail the upload (D6).
- **A new endpoint returns one Asset with its facets.** `GET /api/assets/{id}`
  returns the Asset entry plus each facet's label, value and unit, behind the
  existing session guard. It returns no provenance to the page (D10).
- **The web app gains an asset page.** Nova clicks an image in the Library to
  open `/assets/<id>`. The page shows the image, its name and each facet, such
  as `File size  1,234,567 bytes` in the mono face, with a way back to the
  Library. It says so when no facts are recorded (D6, D7). The address survives
  a reload: the server answers `/assets/<id>` with `index.html`, and no other
  unknown path changes (D8). An upload that is still sending when Nova opens
  an image keeps going, and the Library shows it again on the way back (D12).
- **The page's look is drafted as HTML first.** The user picks one draft, it
  goes into Penpot, and DESIGN.md records the choice before the page is built
  (D9).
- The story's arch-lint scope is already met: `server/scripts/check-arch.sh`
  covers `packs/image` and refuses pack-to-pack dependencies, from #521. This
  change keeps it passing.
- Out of scope, in their own stories: format and dimensions (#525), previews
  (#526), processing goals and their states (#527), and plugin installation
  with an approval step (#361).

## Capabilities

### New Capabilities

- `asset-facets`: the Operator contract and its Asset context, facet schemas
  and grants, the executor that checks proposals, operation runs, stored facet
  values with provenance, and the file-size Operator that upload runs.
- `asset-page`: the signed-in asset page at `/assets/<id>`: opening it from
  the Library, showing the image, name and facets, its loading, missing,
  failed and no-facts states, the way back, and an upload that goes on while
  the page shows.

### Modified Capabilities

- `asset-library`: add `GET /api/assets/{id}`, which returns one Asset with its
  facets, and put it under the existing session guard.
- `image-upload`: a successful upload also runs the file-size Operator before
  it answers, and a failed run still leaves the upload successful.
- `compose-instance`: the web shell answers `/assets/<id>` with `index.html`
  when `<id>` is a well-formed Asset id; every other unknown or folder path
  still gets 404. A first start of the instance holds five migration records.
- `postgres-foundation`: migration 5 adds the facet and operation-run tables,
  so first startup holds five migration records, with an upgrade scenario from
  four.

## Impact

- `server/crates/plugin-abi`: the Operator trait, its Asset context, facet
  schema and grant declarations, and the ABI version.
- `server/crates/core`: migration 5 and its schema snapshot, the executor,
  facet and operation-run persistence, reading an Asset's facets, and the
  user's id on the `User` that the session lookup returns.
- `server/crates/packs/image`: the file-size Operator and its facet schema.
- `server/crates/packs/astro`: the plate-solve stub adapts to the new trait.
- `server/crates/server`: run the Operator after ingest, the new
  `GET /api/assets/{id}` route, and `index.html` for `/assets/<id>`.
- `web/src`: client-side navigation between the Library and the asset page,
  the upload state moved from the Library to the signed-in view, and the page
  itself. The built files stay under `/assets/` (design
  decision 7).
- `DESIGN.md`: an asset page component section, and any token the chosen draft
  needs, regenerated into `tokens.css`.
- `scripts/instance-verify.sh`: expects five migrations.
- Tests: plugin-abi contract tests, core executor and persistence tests against
  PostgreSQL, server route tests, web unit tests, and the browser journey.
