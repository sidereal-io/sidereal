---
name: lets-write-tasks
description: Use when turning an approved plan into an OpenSpec task list. Says how to size, order, tag and verify tasks so that an agent can build them one at a time, test-first. The schema says the checkbox format the apply step parses.
---

# Writing tasks

A task list is the contract between planning and building. An agent with
none of the planning conversation picks up one task at a time. So each
task stands alone: what to build, and how to know it is done.

## Size

- **Five to twelve tasks.** Fewer, and the tasks hide too much. More, and the story was too big: say so, and name the slice to move to a new story, but do not split the story here. That decision belongs to the user and the brainstorm.
- **One task is one red-green cycle:** one behavior, its test, and the code that passes it. A task that needs "and" to describe is two tasks.
- **Minutes, not hours.** Split a task you cannot verify with one command.

## Order

1. **Shared contracts first.** Types, schemas, migrations and interfaces that other tasks depend on, in one sequential section.
2. **Independent work next.** Group tasks that touch different code into sections, and mark each section with the section it depends on.
3. **Integration last.** End-to-end verification that depends on all the others.

A small change has one section. Do not invent parallel sections to fill a shape.

## Tags

Put tags after the task number and before the description.

| Tag | Means |
|---|---|
| `[easy]`, `[medium]`, `[complex]` | How hard the task is. The apply step may use it to choose who does the task. |
| `[parallel]` | The task shares no files with its neighbours, so it can run beside them. |
| `[depends-on: <section>]` | On a section heading: this section waits for that one. |

## Verification

Every task ends with `(verify: <command>)`. A good verify command:

- is one command that exits non-zero on failure, such as `pnpm test -- export` or `cargo test parser`;
- runs the test the task adds, not the whole suite, unless the task is the integration task;
- is one the project already has. Do not invent tooling in a task list.

## Test-first tasks

Write each task so that the agent can write its test first. "Add a CSV
export endpoint" names a feature. "Return a CSV file from GET /export for a
signed-in user" names a behavior a test can state before the code exists.
Name the behavior, not the file.

## Stop conditions

If, while planning tasks, you find a decision that is costly to reverse that
no earlier artifact makes, stop and tell the user. The schema that called
this skill says where the decision goes: the design, when the flow has one,
or the user's answer, when it does not. Never bake the decision into the
task list.
