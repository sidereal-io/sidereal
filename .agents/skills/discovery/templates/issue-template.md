# Issue templates

Two shapes. Every field is required unless marked optional.

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
