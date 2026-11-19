<!-- Write for the people and agents who use the backlog. Explain each journey
in the numbered text as well as the diagram. Use the agreed persona and stage
names, and keep status and issue links beside the stage they describe. -->

# Discovery: <product name>

> The living map behind the backlog: who the product serves and the journeys it supports.
> Stories are GitHub issues; each discovery run's scope and decisions live in its parent issue.

## Personas

### People who use <product name>

#### <Persona name> — <short role>

- **Who**: <who they are>
- **Goal**: <what they're trying to reach>
- **Pain today**: <their real-world problem without a good tool — no code, milestones, or epics>
- **Success looks like**: <the lasting, observable signal they got value>

<!-- In a repository with several products, a persona that more than one
product serves lives in the root map. Name it here with a link instead of a
copy (optional): -->

#### <Persona name> — shared

See [<Persona name>](../discovery.md#<anchor>) in the root map.

### People who build <product name>

#### <Persona name> — <short role>

- **Who**: <who they are>
- **Goal**: <what they're trying to reach>
- **Pain today**: <their real-world problem without a good tool>
- **Success looks like**: <the lasting, observable signal they got value>

> **Implication**: <for personas who share a journey — what that means for the build>

## Journey Map

Stage status checked against the code on YYYY-MM-DD.

### People who use <product name>

**<Journey name>** (<Persona name>):

```
  <Stage 1> ──► <Stage 2> ──► <Stage 3> ──► <Stage 4>
      │             │             │             │
  supported      partial         gap           gap
```

1. **<Stage>** — <what happens> — supported
2. **<Stage>** — <what exists, what's missing> — partial ([#N](https://github.com/<owner>/<repo>/issues/N))
3. **<Stage>** — <what's missing> — gap ([#N](https://github.com/<owner>/<repo>/issues/N))
4. **<Stage>** — <what's missing; no story yet> — gap

### People who build <product name>

**<Journey name>** (<Persona name>, <Persona name>):

<same shape as above>

## Backlog

Stories are GitHub issues: <https://github.com/<owner>/<repo>/issues>.
