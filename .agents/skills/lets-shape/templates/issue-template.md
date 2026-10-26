# Issue templates

One shape for each Kind. Every field is required unless marked optional.

## Parent issue (one per discovery run)

Title: `Epic: <capability>`

```markdown
> Discovery status: in-progress — next phase: <personas | journey-map | moscow | stories>
> Input: <file path, issue link, or "conversation, YYYY-MM-DD">

## Scope

<the confirmed scope, restated in your own words>

## MoSCoW

### Must
- <capability> — <why, tied to a persona's journey>

### Should
- <capability> — <why>

### Could
- <capability> — <why>

### Won't (this release)
- <capability> — <why it's deferred>

## Shared decisions

- <a decision every story in this run relies on> (optional)

## Open questions

- <question, until resolved> (only while the run is in progress)
```

When the run finalizes, remove the status line (keep the `Input:` line) and the Open
questions section, which must be empty by then.
GitHub lists the sub-issues itself; don't repeat them in the body.

## Story issue

Title: `<kebab-story-name> — <one-line outcome>`

```markdown
<one-line outcome>

- **Persona served**: <persona>
- **Journey segment**: <stage(s) covered>
- **MoSCoW**: <Must | Should | Could> — <why>
- **Why this story / why now**: <rationale; the first story is the walking skeleton>
- **Depends on**: <#N, #N — or "nothing">
- **Scope**: in: <2-5 items> / out: <1-2 items>
- **Relevant code**: <paths / modules found during exploration, or "new">
```

When the run fits one story, the parent issue takes this shape instead, keeping the
status line and input reference above the outcome until the run finalizes.

## Bug issue

Title: `<kebab-bug-name> — <one-line outcome>`

```markdown
<one-line outcome: what works once this is fixed>

- **Expected**: <what should happen>
- **Actual**: <what happens, with the error text if there is one>
- **How to reproduce**: <steps, or the command>
- **Relevant code**: <paths / modules, or "unknown">
- **Scope**: in: <1-3 items> / out: <1-2 items>
```

## Chore issue

Title: `<kebab-chore-name> — <one-line outcome>`. A chore has no template: its body is the one-line outcome.

## Glossary recovery record (optional for every Kind)

Append this section to a parent, story, bug or chore only when shaping has
owned glossary edits or the packet references their owner. Preserve an
existing record verbatim when rebuilding any issue body, including when a
parent becomes a story. Update its confirmed facts without dropping the
checkpoint history. Omit the section when there is no glossary handoff.
A dependent packet can link to the complete owner record in **Depends on**
instead of repeating it, but must retain any unresolved prerequisite.

```markdown
## Glossary handoff

- **Owner**: <issue and PR, or explicitly undecided>
- **Owned paths**: <exact project glossary paths>
- **Starting branch and base**: <branch and full commit>
- **Remote and branch**: <remote URL and checkpoint branch, or local-only>
- **Checkpoints**: <ordered full commit IDs and their parents; recorded head>
- **Publication**: <verified remote head, or unavailable with reason>
- **Prerequisites**: <dependent stories, confirmed blocker links or declined links with unresolved requirement>
- **Destination**: <branch, imported commits and PR when available; otherwise pending>
```

Do not copy definitions into this record. Recover their committed contents
from the checkpoint before resuming work.
