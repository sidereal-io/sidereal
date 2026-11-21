# Records

Read this file when the walk prints a Skip line, or when the records
disagree.

## Read the records

One read dates each skip line. `<date>` is the commit's author date,
from `%as`, as `YYYY-MM-DD`. A rebase keeps it. `<sha>` is
the short hash, from `%h`. A read that prints nothing means the
[records disagree](#records-that-disagree).

- **Shape.** [The step lines](../SKILL.md#the-step-lines) says how to
  date shape's line.
- **Plan.** Read the commit that added the task list:

  ```bash
  git log -1 --diff-filter=A --format='%h %as' -- openspec/changes/<name>/tasks.md
  ```

- **Review plan.** On `lets-minimal`, there is no record, and the line
  has no date. On any other schema, the `lets-review-plan` skill records
  the review in the pull request's `body`. Find the `## Plan review`
  heading, and read the section up to the next `## ` heading or the end
  of the body. Inside it, `<sha>` comes from the line that starts
  `Reviewed: `, and `<verdict>` from the line that starts
  `- **Verdict:** `. Do not search the whole body: a **Code review**
  section has a `Reviewed:` line too. `<pr>` is the pull request's
  `number`. Date the line by the reviewed commit:

  ```bash
  git log -1 --format=%as <sha>
  ```

- **Build.** Read the last commit that changed the task list: the last
  task's tick.

  ```bash
  git log -1 --diff-filter=M --format='%h %as' -- openspec/changes/<name>/tasks.md
  ```

- **Review build.** Read the commit that archived the change. `<pr>` is
  the pull request's `number`.

  ```bash
  git log -1 --diff-filter=A --format='%h %as' -- openspec/changes/archive/<YYYY-MM-DD>-<name>/tasks.md
  ```

The plan and build reads work after archive too. Archive removes the
old path, and its history keeps the add and the ticks.

## Records that disagree

When the records disagree, print `Records disagree:` and what you found.
Say what to fix by hand, and stop before any step. Repair nothing: each
repair means choosing which record is right, and that is the user's
choice.

| What you found | What to fix by hand |
|---|---|
| `git status --porcelain` lists a path outside `.workspace/` | Commit or discard the path. |
| The branch and `origin/<name>` have diverged | Reconcile the two, and push. |
| A change folder and an archive folder for the same name | Keep the folder that is right, and remove the other. |
| An archive folder, and the pull request is missing or a draft, or has no **Code review** section on `lets-standard` | Open the pull request, review it, or mark it ready, as `lets-review-build` does. Or revert the archive commit. |
| `brainstorm.md` is committed, and the branch has no pull request | Push the branch, and open the draft pull request. |
| An archived change on a schema other than `lets-minimal`, and the pull request's body has no **Plan review** section with a `Reviewed:` line and a `- **Verdict:**` line | Review the plan with the `lets-review-plan` skill, which writes the section. Or revert the archive commit. |
| A `git log` read for a skipped step finds no commit | Name the command, and find out why the step's record is missing. |

The printed line names what you found, such as the path, or the pull
request's `isDraft`. A built change that is not archived,
and whose pull request has no **Plan review** section, is not a
disagreement. The walk reaches review plan first, and runs it.
