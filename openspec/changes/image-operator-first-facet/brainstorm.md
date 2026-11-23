<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#524 image-operator-first-facet — an uploaded image shows its file size, written by a plugin through the public contract. https://github.com/sidereal-io/sidereal/issues/524

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Keep the Operator contract, executor, facet and run tables, file-size Operator and asset page as one story, or move the asset page to its own story? | Keep it as one story. | User |
| D2 | Which uploads does the file-size Operator run on? | Every new image upload. No backfill: images already in the library show no facts. | User |
| D3 | What is the facet called, and what does it describe? | `image.file-size`, a whole number of bytes, on the asset version. It repeats `asset_versions.byte_size` on purpose, to prove the contract with the smallest fact. | User |
| D4 | How does a built-in Operator get permission to write a facet? | The Operator lists the facets it writes when its pack registers. Core treats a pack built into the server as already approved, until plugin installation (#361) adds an approval step. | User |
| D5 | When does the Operator run after an upload? | Inside the same upload request, after the asset is saved. The upload answers once the size is recorded. | User |
| D6 | What happens when the Operator fails, or core refuses what it proposed? | The upload still succeeds. The run is recorded as failed with its reason. No facet is written. The asset page shows the image with a line saying no facts are recorded. | User |
| D7 | What does the asset page show, and how does Nova reach it? | Nova clicks an image in the Library to open its page. The page shows the image, its name and each facet as its schema's label, value and unit (for example `File size  1,234,567 bytes`, in the mono font), with a way back to the Library. | User |
| D8 | What address does the asset page have? | A real `/assets/<id>` address that survives a reload. The server serves `index.html` for that path only. | User |
| D9 | How is the asset page's look chosen? | Draft it here as HTML first; the user picks one; then it goes into Penpot. | User |
| D10 | Does the asset page show who produced each fact (plugin, version, time)? | Only the value. Core still stores the provenance, as ADR-008 requires. | User |
| D11 | Should *Facet schema* and *Grant* go in the glossary under "Assets and processing"? | Yes, as worded. Both are now in `openspec/glossary.md`. | User |

## Changes from the story

- **Added: the server serves `index.html` for `/assets/<id>`** (D8). The packet did not name a server route change; a real page address needs one.
- **Added: two glossary terms, *Facet schema* and *Grant*** (D11). The packet said the glossary terms land with #521; these two are new and land with this change.
- **Added: HTML drafts, then Penpot, for the asset page** (D9). The page is the web app's second screen and has no design yet.
- **Clarified: no backfill** (D2). Images uploaded before this change show no facts.

Nothing was cut or split out.

Confirmed by user: no
