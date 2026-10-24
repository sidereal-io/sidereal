---
name: lets-write-specs
description: Use when writing or changing an OpenSpec spec. Says what belongs in a requirement, how to write a scenario a test can assert, and how to change an existing requirement without losing detail. The schema says the file layout and the parser's format.
---

# Writing specs

A spec is a behavior contract, not an implementation plan. It says what the
system does as seen from outside. The design says how.

## Read and maintain domain language

Read [Domain language](../lets-shape/references/domain-language.md) before
using domain terms. Resolve project vocabulary from `planningHome.root` in
OpenSpec's artifact instructions, including a selected planning store. Read
the context-selected shared glossary plus existing ancestor and local
glossaries for all affected full capability paths. Do not assume an earlier
session loaded them. Missing optional files need no empty replacements;
a missing declared pointer needs discussion before dependent work proceeds.

Use the agreed concepts consistently in requirements and scenarios. Surface
semantic contradictions before writing dependent requirements; qualify
explicit scoped meanings instead of letting a local definition override an
ancestor. If new meaning is needed, settle it with the user and update the
authoritative glossary outside change deltas as a separate `docs:` unit.
When retiring a capability, review actual remaining uses: move surviving
definitions to the appropriate remaining scope, update references, and
remove obsolete definitions explicitly. Spec retirement does not authorize
deleting its glossary. When removing a capability's final requirement, set
`retire_capabilities: true` alongside `schema:` in the change's
`.openspec.yaml` before archive. Glossary cleanup remains an explicit author
decision; this option retires the spec, not its glossary.

## What belongs in a spec

- Observable behavior that users or downstream systems rely on.
- Inputs, outputs and error conditions.
- External constraints: security, privacy, reliability, compatibility.
- Scenarios a test can check.

## What does not

- Internal class, function or module names.
- Library or framework choices.
- Step-by-step implementation.
- Execution plans. Those belong in the design and the tasks.

The test: if the implementation could change without changing what anyone
outside can observe, it does not belong in the spec.

## Requirements

- Write normative text with SHALL or MUST. Avoid should and may; they leave the requirement untestable.
- One requirement states one behavior. Split a requirement that needs "and" to describe.
- Every requirement has at least one scenario.

## Scenarios

A scenario is a potential test. Its WHEN names a concrete condition. Its THEN
names an outcome a test can assert: a value, a message, a state, a status
code. "THEN the system handles it correctly" asserts nothing.

```markdown
#### Scenario: Known user opens the app
- **WHEN** a user named Ada, who has signed in before, opens the app
- **THEN** the app shows "Hello, Ada"
```

Cover the failure path as well as the success path when the requirement has
one: the missing input, the denied request, the timeout.

## Changing an existing requirement

### Choose the capability paths

Use `planningHome.root` from `openspec instructions ... --json` to locate the
main specs under `openspec/specs/`. A capability path is the directory relative
to that spec folder, such as `user-auth` or `identity/user-auth`. Preserve the
full path in the change's `specs/<capability-path>/spec.md`.

Before writing a delta for an existing capability, confirm its full ID with
`openspec list --specs` and read it with `openspec show <spec-id> --type spec`.
This includes MODIFIED, REMOVED and RENAMED requirements. Do not shorten an ID
to its leaf name: strict validation can pass a wrong path that archive later
refuses. For new capabilities, follow the project's existing layout and check
that the behavior is not already covered under another path.

Read shared specs the change depends on, but write a delta only when their
requirements change. One change can carry deltas for several domains. Leave
specs read only for context out of the change's delta files. Keep flat paths
flat; grouping by domain is optional and needs no migration or registry.
Domain names do not map to source folders, so find the specs a change
touches from the project context, the request and the code.

### Preserve the requirement

Archive replaces the whole requirement with what the delta holds. So:

- **MODIFIED** carries the entire requirement, from its `### Requirement:` heading through every scenario, with the edits made. Partial content loses the rest at archive.
- **ADDED** is for a new requirement, including a new concern on an existing capability that leaves its other requirements alone. Prefer ADDED over MODIFIED when you are adding, not changing.
- **REMOVED** names the requirement, the reason, and the migration for anyone who relied on it.
- **RENAMED** changes only the name. Use MODIFIED if the behavior changes too.

Keep the heading text of a MODIFIED or REMOVED requirement identical to the
existing one, so archive can find it.

## Example: a new capability

A new capability opens with its Purpose and has only ADDED requirements.
There is nothing yet to modify or remove.

```markdown
## Purpose

Lets users take their data out of the product in a portable format.

## ADDED Requirements

### Requirement: User can export data
The system SHALL let a signed-in user download all their data as one CSV file.

#### Scenario: Successful export
- **WHEN** a signed-in user chooses Export
- **THEN** the browser downloads a CSV file that holds every record the user owns

#### Scenario: Export while signed out
- **WHEN** a request to export arrives without a session
- **THEN** the system responds 401 and sends no data
```

## Example: a delta on an existing capability

A delta has no Purpose. It changes, adds or removes requirements the main
spec already holds.

```markdown
## MODIFIED Requirements

### Requirement: User can export data
The system SHALL let a signed-in user download all their data as one CSV or
JSON file, as they choose.

#### Scenario: Successful export
- **WHEN** a signed-in user chooses Export and picks a format
- **THEN** the browser downloads one file in that format that holds every record the user owns

## REMOVED Requirements

### Requirement: Legacy export endpoint
**Reason**: Replaced by the format-aware export.
**Migration**: Call `/api/v2/export?format=csv` instead of `/api/v1/export`.
```
