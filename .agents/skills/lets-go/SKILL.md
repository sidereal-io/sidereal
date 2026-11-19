---
name: lets-go
description: Use when the user wants a story, bug or chore, or an idea in words, built end to end to a pull request ready for review; wants the next story; wants to carry on the change on the current branch; says lets-go; or runs it again after a run stopped. It runs each step in a fresh session.
---

# Let's go

Take one story, bug or chore to a pull request ready for review, in one
run. Start from its issue number, from an idea in words, or from
nothing, which takes the next ready issue. The run claims the issue in
this session. Then each [step](references/terms.md#step) runs in a
fresh session: shape when the issue needs it, then plan, review plan,
build and review build. `lets-minimal` skips review plan. An idea too
large for one issue runs discovery in the shape step, and the run builds
the first ready story it files. A step never asks the user: it ends with
a [step report](references/terms.md#step-report). This session answers
the step's questions or asks the user, as the
[question mode](references/terms.md#question-mode) says.
"You" is the agent that runs this skill, and "the user" is the person
who asked for the run. Run every command below from the repo root.

A run can stop before it finishes. Run `lets-go` again to go on. It
reads from Git and GitHub which steps are done. What the new run does
depends on where you run it, and on why the last run stopped:

- **On the change's branch:** it finds the issue from the branch's
  name, prints a dated line for each step already done, and goes on
  from the first unfinished step.
- **On the default branch, with the issue number:** it finds the
  change's branch, here or on `origin`, switches to it, and goes on the
  same way. It does not claim the issue again.
- **On the default branch, with nothing:** it does not resume. It picks
  the next ready issue, and a claimed issue is never ready. To resume,
  give the issue number, or run it on the change's branch.
- **After a failed step:** the run that failed never runs that step
  again. A new run starts it afresh, in a new session, with the answers
  given so far. A failed shape step has no branch: the run says to run
  `lets-shape` on the idea or the issue.
- **After the pull request is merged or closed:** there is nothing to
  resume, and the run stops.

Read [Plain language](../lets-shape/references/plain-language.md)
before writing.
Use it for questions and progress reports. Keep the fixed report formats
below; explain their evidence and reasons in plain words.

## The run at a glance

Start at [Read the settings](#read-the-settings). The table below shows
the route through the run; each linked part gives its detailed actions.
A run goes through these parts, in this order.
[Start from an idea](#start-from-an-idea),
[Run the shape step](#run-the-shape-step) and
[Pick the first ready story](#pick-the-first-ready-story) run only when
the request or the [packet](references/terms.md#packet) needs them. Each
part runs in this session. A part is not a step, and has no step report.
The shape step that [Run the shape step](#run-the-shape-step) runs is a
step, and has a report.

Some parts run a [procedure](references/terms.md#procedure), a section
under [Procedures](#procedures). A part or another procedure runs it,
and it returns its result to what ran it.
[Find the source of demand](#find-the-source-of-demand) finds the run's
issue: the one the user named, the one an idea's words describe, or
`next`. `next` is the first ready issue on the backlog, as that
procedure ranks it.

| Part | What it does | Runs in | Next | Stops when |
|---|---|---|---|---|
| [Read the settings](#read-the-settings) | Reads the default question mode, the default branch and the current branch | this session | [Read the request](#read-the-request) | `lets go questions` fails |
| [Read the request](#read-the-request) | Takes a question mode off the request, after asking the user when the mode word sits beside an idea, then finds the issue from the branch and the request | this session | [Check the issue](#check-the-issue), or [Start from an idea](#start-from-an-idea) for an idea | An idea on a change's branch, an idea with no letter or digit, no issue that matches the branch, or [Find the source of demand](#find-the-source-of-demand) returns `nothing-ready` |
| [Start from an idea](#start-from-an-idea) | Resumes a discovery of the idea, or searches for an issue that is the same work | this session | [Check the issue](#check-the-issue) for the same work; otherwise [Run the shape step](#run-the-shape-step) | Never |
| [Check the issue](#check-the-issue) | Reads the blockers, looks for the change's branch, finds the source of demand, confirms the packet and prints shape's line | this session | [Run the shape step](#run-the-shape-step) when the packet needs shaping and shape can run; otherwise [Route](#route) | An open blocker with no branch; [Find the source of demand](#find-the-source-of-demand) returns `unresolved`; or [Confirm the packet](#confirm-the-packet) returns `epic`, or `needs-shape` when shape cannot run |
| [Run the shape step](#run-the-shape-step) | Runs `lets-shape` on the idea or the issue | this session; its step runs in a fresh one | [Check the issue](#check-the-issue) for one issue; [Pick the first ready story](#pick-the-first-ready-story) for a parent and its stories | The discovery map has edits of its own, or the step fails or asks a settled question again |
| [Pick the first ready story](#pick-the-first-ready-story) | Picks the first ready story that discovery filed | this session | [Check the issue](#check-the-issue) | [Find the source of demand](#find-the-source-of-demand) returns `nothing-ready` |
| [Route](#route) | Picks the change's schema, and says the route line | this session | [Enter the branch](#enter-the-branch) | Never |
| [Enter the branch](#enter-the-branch) | Claims the issue and creates the branch, or switches to the branch and brings it up to date, then sets the schema | this session | [Set the question mode](#set-the-question-mode); [Read the request](#read-the-request) or [Pick the first ready story](#pick-the-first-ready-story) after a lost claim | [Claim](#claim) returns `unresolved`, `checkpoint-failed`, `claim-failed` or `branch-failed`, or `taken` for a named issue or for the third time; the import returns `import-failed`; or the branch has diverged |
| [Set the question mode](#set-the-question-mode) | Picks `ask` or `auto`, and says why | this session | [Walk the step table](#walk-the-step-table) | Never |
| [Walk the step table](#walk-the-step-table) | Reads the records, then runs each step that is not done | this session; its step runs in a fresh one | [End of the run](#end-of-the-run) | The records disagree, the pull request is merged or closed, or a step fails, reports done when its record says otherwise, or asks a settled question again |
| [End of the run](#end-of-the-run) | Prints the pull request's URL | this session | Nothing: the run is over | Always |

Any part also stops when a `lets backlog` command returns an error, as
[Backlog errors](#backlog-errors) says.

## Placeholders

A name in angle brackets is a placeholder. Each one stands for one
thing, the same everywhere in this skill. Fill it in from where the run
sets it.

`lets backlog` names each item by its `key`, and shows it by its
`label`: `149` and `#149` on GitHub. In anything shown to a person,
`#<i>`, `#<parent>` and `#<story>` stand for the item's `label`: a
printed line, a question, or text written in an issue. In a command,
`<i>`, `<parent>` and `<story>` stand for its `key`. With the GitHub
backlog provider, the label is `#` and the number, so each line reads
as it always has.

| Placeholder | What it stands for | Where the run sets it |
|---|---|---|
| `<i>` | The run's issue, as its key: the key the user named, the issue `next` or [Pick the first ready story](#pick-the-first-ready-story) picked, or the issue a shape step shaped. Or, inside `next`, the candidate it is checking | [Read the request](#read-the-request), the shape step's report, or `next`, for each candidate |
| `<issue-url>` | The URL of `<i>` | [Confirm the packet](#confirm-the-packet), from the `url` that `lets backlog item <i>` prints |
| `<parent>` | A parent issue, as its key: an epic that discovery filed, or a discovery in progress | The shape report's `epic` line, or the discovery query in [Start from an idea](#start-from-an-idea) |
| `<story>` | One story that a shape step's report names, as its key | Each `story` line of the shape report |
| `<paths>` | The changed discovery maps, one path each | [Commit the discovery map](references/shape.md#commit-the-discovery-map), from its `git status` command |
| `<pr>` | The number of the change's pull request | [Read the records](references/records.md#read-the-records), from the pull request's `number` |
| `<name>` | The change's name: the part of `<i>`'s title before ` — `. On a change's branch, it is `<branch>` | [Check the issue](#check-the-issue), from the blocker read's title. After a shape step, [Check the issue](#check-the-issue) sets it again from the shaped issue's title |
| `<branch>` | The branch the run started on, when it is not the default branch | [Read the settings](#read-the-settings), from `git branch --show-current` |
| `<default>` | The repository's default branch | [Read the settings](#read-the-settings) |
| `<shape-name>` | The shape step's name: `issue-<i>` or `idea-<slug>` | [The shape step's name](references/shape.md#the-shape-steps-name) |
| `<slug>` | The first five words of the idea, as [The shape step's name](references/shape.md#the-shape-steps-name) makes them | [The shape step's name](references/shape.md#the-shape-steps-name) |
| `<claim>` | The unique token of this session's claim | [Claim](#claim), from the command's `claim` field |
| `<idea>` | The idea, in the user's words | [Read the request](#read-the-request) |
| `<word>` | The one word left of the request, before `lets backlog key` says whether it names an issue | [Read the request](#read-the-request) |
| `<step>` | One step, as `--for` names it: `shape`, `plan`, `review-plan`, `build` or `review-build` | [The step table](#the-step-table), at the row the walk runs, or `shape` in [Run the shape step](#run-the-shape-step) |
| `<session>` | A step's session label: `<name>`, a hyphen and `<step>`, or `<shape-name>-shape` for shape | [The change's name](#the-changes-name) |
| `<mode>` | The question mode, `ask` or `auto` | [Set the question mode](#set-the-question-mode), or [Run the shape step](#run-the-shape-step), before shape |
| `<Kind>` | The issue's Kind on the Project | [Confirm the packet](#confirm-the-packet) |
| `<schema>` | The change's schema | [Enter the branch](#enter-the-branch) |
| `<route line>` | The line [Route](#route) says, such as `Route: lets-minimal, because #<i> is a Chore.` | [Route](#route) |
| `<title>` | The title of the issue the line names | The command or report line that names the issue |
| `<date>` | The date on a printed line | [The step lines](#the-step-lines), for each kind of line |
| `<sha>` | A commit's short hash, or the commit a review names | [Read the records](references/records.md#read-the-records), or the review plan report |
| `<verdict>` | A plan review's verdict | The **Plan review** section, or the review plan report |
| `<evidence>` | What a `done` report's `evidence` lists | [The step lines](#the-step-lines) |
| `<reason>` | Why a step failed | [The step lines](#the-step-lines), from a `failed` report's `reason`, or from [Read the report](#read-the-report) when a `done` report's record says otherwise: `the step reported done, but` and what the record holds |
| `<YYYY-MM-DD>` | The date in an archive folder's name | The folder's name in `openspec/changes/archive/` |
| `<first>` | The first model of the ranked options | [Run a step](#run-a-step), when it ranks the options |
| `<second>` | The second model of the ranked options | [Run a step](#run-a-step), when it ranks the options |
| `<skill>` | The skill in the step's row of the step table | [Write the prompt](references/step-prompt.md#write-the-prompt) |
| `<plain-language-path>` | The absolute path to the installed plain-language reference | [Write the prompt](references/step-prompt.md#write-the-prompt) |
| `<the sentence above>` | The sentence for `## What to run`, or for shape the text of [What shape runs](references/shape.md#what-shape-runs) | [Write the prompt](references/step-prompt.md#write-the-prompt) |
| `<kebab-name>` | The part of an issue title before ` — `, in `lets-shape`'s template | The issue's title |
| `<one-line outcome>` | The part of an issue title after ` — `, in `lets-shape`'s template | The issue's title |
| `<L or XL>` | The Size the shape step gave the work, `L` or `XL` | The shape step |

## Backlog errors

The run reads and claims the backlog through `lets backlog`. Three
reads stay as `gh` commands for now: the comment thread read of
[Confirm the packet](#confirm-the-packet),
`gh issue view <i> --json createdAt`, and the in-progress parent query
of [Start from an idea](#start-from-an-idea). Each `lets backlog`
command prints one JSON object. When the object holds
`error`, the backlog cannot answer. Stop: claim nothing more, and run no
step. Say what the error is:

- **`no-backlog`:** no Project is linked to the repository. Say to run
  the `lets-setup-backlog` skill.
- **`several-backlogs`:** list each Project's `title` and `url` from its
  `projects`, and ask the user to unlink the extras. Never pick one
  yourself.
- **Any other error,** such as `backend-failed` when the backlog
  provider's tool fails, or `unknown-backlog-provider`: show its
  `message`.
  When it carries `claim`, show the token. The user can run
  `lets backlog release <i> --claim <claim>`. Say the issue may still
  hold your assignment or claim comment. Do not retry.

Each part and procedure that runs a `lets backlog` command links here
for its errors.

## Read the settings

Read the user's default question mode. Run this once, before anything
asks GitHub:

```bash
lets go questions
```

It prints one JSON line, such as `{"questions":"auto"}`. Keep its
`questions` for the whole run: `ask`, `auto`, or `null` when the
`lets` config sets none. If it exits non-zero, show the user its
stderr and stop: claim nothing, and run no step. Then find the
default branch with
`gh repo view --json defaultBranchRef --jq .defaultBranchRef.name`,
and run `git branch --show-current`.

Next: [Read the request](#read-the-request).

## Read the request

Read the request. A leading or trailing `ask` or `auto` may name the
question mode. When the rest of the request is a number or nothing, the
word names the question mode: take it off, and keep it for
[Set the question mode](#set-the-question-mode). When the rest is
words, on the default branch, ask the user whether the word names the
question mode or is part of the idea. Show both readings: for
`ask before deleting the branch`, the question mode `ask` with the idea
`before deleting the branch`, or the idea
`ask before deleting the branch`. Ask it in either mode, before
[Start from an idea](#start-from-an-idea) runs: it asks what the
request says, not which question mode to use. When the user says the
word names the question mode, take it off, and keep it for
[Set the question mode](#set-the-question-mode). Otherwise the word
stays in the idea, and [Set the question mode](#set-the-question-mode)
takes the question mode from its next source. On another branch, ask
nothing about the word: words stop the run there.

What is left is one of these:
- **A number:** one word that is one of the backlog provider's item
  keys, such as `#149` or `149` on GitHub, or `PROJ-12` on Jira. It
  names the issue.
- **Words:** anything else. They are an idea, kept word for word.
- **Nothing.**

When what is left is one word, ask `lets backlog key` whether it is one
of the backlog provider's item keys. Pass the word as one argument in
single quotes, after `--`, and write each `'` in it as `'\''`:

```bash
lets backlog key -- '<word>'
# example
lets backlog key -- 'can'\''t'
```

When it prints a `key`, the word names an issue, and that `key` is
`<i>`. When `key` is `null`, the word is words, such as `refactor`.
Ask it before the discovery query and the title search. On an error,
act as [Backlog errors](#backlog-errors) says.

Then find the issue, `<i>`, from the branch the run started on and what
is left of the request:

| The branch | A number | An idea | Nothing |
|---|---|---|---|
| The default branch | The number is `<i>`. Go to [Check the issue](#check-the-issue) | With no letter or digit, stop. Otherwise go to [Start from an idea](#start-from-an-idea) | Run `next`. Its pick is `<i>`: print the `Picked` line, and go to [Check the issue](#check-the-issue). When nothing is ready, stop |
| Another branch, `<branch>` | When the number is one of the matches, it is `<i>`: go to [Check the issue](#check-the-issue). Otherwise stop | Stop: an idea needs the default branch | When exactly one issue matches, it is `<i>`: go to [Check the issue](#check-the-issue). When none or several do, stop |

With an idea on the default branch, read its characters first. When
the idea holds no letter or digit, in any script, say that an idea
needs at least one letter or digit, and stop, claiming nothing and
running no step. This comes before the discovery query and the title
search, which an idea like `???` gives nothing to work with. A letter
in any script counts: `ログを直す` is an idea.

With nothing on the default branch, run the `next` case of
[Find the source of demand](#find-the-source-of-demand), over the
whole backlog. On its result:
- **`picked`:** print:

  ```text
  Picked #<i>: <title>, the next ready issue.
  ```

  That issue is `<i>`.
- **`nothing-ready`:** say the backlog has nothing ready, suggest the
  `lets-shape` skill to shape more, and stop, claiming nothing.

On another branch, `<branch>`, when the request is words, say that an
idea needs the default branch, and stop. Otherwise search for the
change's issue, once. On an error, act as
[Backlog errors](#backlog-errors) says.

```bash
lets backlog find <branch>
```

Keep the `issues` whose title starts with `<branch> — `: these are the
matches. When the user gave a number, `<i>` must be the `key` of one
of them. If it is not, say that the issue and the branch do not match,
and stop. When they gave none, exactly one match is the issue. If none
matches, say that `lets-go` runs on the default branch or on a change's
branch, and needs the issue number on a change's branch. Stop. If
several match, say that `lets-go` needs the issue number, and stop.

Next: [Check the issue](#check-the-issue) when the entry table found
`<i>`; [Start from an idea](#start-from-an-idea) for an idea on the
default branch.

## Start from an idea

Follow its steps in
[the shape file](references/shape.md#start-from-an-idea).

Next: [Check the issue](#check-the-issue) when the user says an issue is
the same work; otherwise [Run the shape step](#run-the-shape-step).

## Check the issue

1. Read the blockers of `<i>`. On an error, act as
   [Backlog errors](#backlog-errors) says.

   ```bash
   lets backlog item <i>
   ```

   Its `blockedBy` lists each blocker with its `state`, and it gives the
   `title` too. Keep the whole result: the packet check below reads it.
   `<name>` is the change's name, which [Claim](#claim) takes from the
   title. On a change's branch, it is `<branch>`.
2. Look for the change's branch, here and on `origin`, every time:

   ```bash
   git branch --list <name>
   git ls-remote --heads origin refs/heads/<name>
   ```

   On a change's branch, the branch exists: you are on it. Run the
   `git ls-remote` there too, to learn whether `origin` has it. Keep
   what you found: [Enter the branch](#enter-the-branch) reads it, and
   does not look again.
3. Decide on an open blocker before anything else in this part runs. A
   blocker whose `state` is `OPEN` is open. When one is:
   - **The branch exists**, here or on `origin`: the user has stacked it
     by hand. Name the open blockers by number, and go on.
   - **It does not exist:** say which open issues block `#<i>`, and
     stop. The user stacks it on the blocker's branch by hand.

   Either way, name a blocker in another repository by its `url`, not
   its number: a bare number means an issue of this repository.
4. Run [Find the source of demand](#find-the-source-of-demand) for
   `<i>`. Its first case runs
   [Check the prerequisite](references/glossary-handoff.md#check-the-prerequisite)
   for the issue. On its result:
   - **`found`:** go on to [Confirm the packet](#confirm-the-packet),
     below.
   - **`unresolved`:** report the unresolved owner or checkpoint it
     carries, and stop, even on a stacked branch.
5. Run [Confirm the packet](#confirm-the-packet). On its result:
   - **`complete`:** go on to the shape line, below. For a Story with a
     parent, the result carries the parent's **Shared decisions**:
     [Route](#route) needs them.
   - **`needs-shape`:** [run the shape step](#run-the-shape-step) for
     `#<i>` when all of these hold:
     - the user named `<i>`, or said in
       [Start from an idea](#start-from-an-idea) that it is the same
       work;
     - the branch lookup above found no branch, here or on `origin`;
     - no shape step has run in this run.

     Otherwise, when the branch exists, first say that the change has
     started on `<name>`. Say why the packet needs shaping, as the
     result carries it, and show nothing of what a comment says. Say to
     run the `lets-shape` skill on `#<i>`, and stop.
   - **`epic`:** say that an epic is built through its stories, list the
     open sub-issues the result carries, and stop. List a sub-issue in
     another repository by its `url`.
6. When [Confirm the packet](#confirm-the-packet) returns `complete`,
   shape is done. Print shape's line, as
   [The step lines](#the-step-lines) says.

Next: [Run the shape step](#run-the-shape-step) when
[Confirm the packet](#confirm-the-packet) returned `needs-shape` and
shape can run; otherwise [Route](#route).

## Run the shape step

Follow its steps in
[the shape file](references/shape.md#run-the-shape-step).

Next: [Check the issue](#check-the-issue) when shape shaped one issue;
[Pick the first ready story](#pick-the-first-ready-story) when it filed
a parent and its stories.

## Pick the first ready story

Follow its steps in
[the shape file](references/shape.md#pick-the-first-ready-story).

Next: [Check the issue](#check-the-issue), with the story that is ready.

## Route

Pick the schema for the change. Top down, first match wins:

| The issue | Schema |
|---|---|
| Kind Chore | `lets-minimal` |
| Size XS or S, and the packet's **Relevant code** says `new` | the flow's schema |
| Size XS or S, and the parent epic's **Shared decisions** names one this story relies on or creates | the flow's schema |
| Size XS or S | `lets-minimal` |
| anything else, including Size unset | the flow's schema |

The flow's schema is the `schema:` line of `openspec/config.yaml`. Say
which schema and why in one line, such as
`Route: lets-minimal, because #<i> is a Chore.` That line goes in the
proposal commit message.

Keep the route line: the plan step needs it.

Next: [Enter the branch](#enter-the-branch).

## Enter the branch

1. On the default branch, use what [Check the issue](#check-the-issue)
   found when it looked for the change's branch, here and on `origin`:
   - **Neither has it:** run [Claim](#claim), which creates the
     branch, and act on its result as below.
   - **Either has it:** the change has started. Do not run
     [Claim](#claim). When `origin` has it, run
     `git fetch origin <name>` first: `git switch` finds a branch on
     `origin` only once it is fetched. Then run `git switch <name>`.

   Stay on `<name>` for the whole run.
2. When you ran [Claim](#claim), act on its result:
   - **`claimed`:** when it carries `verified`, run
     [Import the glossary checkpoint](references/glossary-handoff.md#import-the-glossary-checkpoint)
     with the commits it carries, and act on its result as below. When
     it carries `no-owned-edits`, run no import. Then, when a shape step
     ran in this run,
     [commit the discovery map](references/shape.md#commit-the-discovery-map).
     A run with no shape step has no map to commit, and reads no part of
     `references/shape.md`.
   - **`taken`:** when the user named `<i>` or confirmed it, say who has
     it, and stop. Otherwise `next` or
     [Pick the first ready story](#pick-the-first-ready-story) picked
     it: count the lost claim. After the third lost claim in this run,
     say that other sessions keep taking the picked issues, and to run
     `lets-go` again. Stop, claiming nothing. Before that, go back to
     the part that picked: [Read the request](#read-the-request) for
     `next`, or
     [Pick the first ready story](#pick-the-first-ready-story). Keep the
     lines printed for the lost issue.
   - **`unresolved`:** report the owner or the definitions it carries,
     and stop, claiming nothing, even for an issue that `next` picked.
   - **`checkpoint-failed`:** report what the check found, and stop.
   - **`claim-failed`:** show its message. Report what may remain by
     `write`:

     - `assign`: your assignment; no claim token exists.
     - `claim`: your assignment and possibly a claim comment.
     - `delete-claim`: claim comments and your assignment. After
       release, check that no claim comment or assignment of yours
       remains.
     - `unassign`: your assignment and possibly In Progress Status;
       the user fixes both by hand.
     - `status`: Status, and possibly your assignment and comment.
       With no comment left, the user resets Status by hand.

     Ask the user to inspect the issue. With a claim comment, they can
     run `lets backlog release <i> --claim <claim>`. Do not release it
     yourself. Stop, and run no step.
   - **`branch-failed`:** [Claim](#claim) attempted release. Show
     the branch's error it carries. When it carries a release error
     too, show it, and say the issue may still hold your assignment or
     its In Progress Status. Then stop.
3. When you ran
   [Import the glossary checkpoint](references/glossary-handoff.md#import-the-glossary-checkpoint),
   act on its result:
   - **`imported`:** when a shape step ran in this run,
     [commit the discovery map](references/shape.md#commit-the-discovery-map),
     as for `no-owned-edits`.
   - **`import-failed`:** report each cleanup failure it carries, and
     what was kept, with how to recover, and stop.
4. When `origin` has the branch, bring it up to date before you read any
   record. On a change's branch too, [Check the issue](#check-the-issue)
   found whether it does.

   ```bash
   git fetch origin <name>
   git merge --ff-only origin/<name>
   ```

   Skip the fetch when you have just run it above. If the merge cannot
   fast-forward, the branch and `origin` have diverged: the
   [records disagree](references/records.md#records-that-disagree). If
   any command here fails for another reason, show the user its error,
   and stop.
5. Set `<schema>` from the `schema:` line of the first of these files
   that exists:
   - the change's `openspec/changes/<name>/.openspec.yaml`;
   - the archived change's
     `openspec/changes/archive/<YYYY-MM-DD>-<name>/.openspec.yaml`, in
     the folder whose whole name is `<YYYY-MM-DD>-<name>`.

   When neither exists, it is the schema [Route](#route) picked. Archive
   keeps the file, so an archived change keeps the schema it was built
   with, even when [Route](#route) would now pick another.

Next: [Set the question mode](#set-the-question-mode);
[Read the request](#read-the-request) or
[Pick the first ready story](#pick-the-first-ready-story) after a lost
claim.

## Set the question mode

Set `<mode>`, the question mode. Take the first of these that is set:
1. The question mode the user named in the request that started the
   run.
2. The question mode `lets go questions` printed in
   [Read the settings](#read-the-settings), when it is not `null`.
3. The Kind's: `auto` for a Chore or a Bug, and `ask` for any
   other Kind, a Story included.
   [Confirm the packet](#confirm-the-packet) reads any other Kind as a
   Story. Before a shape step, the Kind does not count, even when the
   Project has one, as for a Bug whose packet lacks a line. So before a
   shape step, the third source is `ask`.

Never ask the user which question mode to use. The question mode holds
for this run only. Do not run `lets go questions` again. Print the line
for the source you used. The last line is for the third source, before
a shape step:

```text
Questions: <mode>, because you asked for it.
Questions: <mode>, because your lets config sets go.questions.
Questions: <mode>, because #<i> is a <Kind>.
Questions: ask, because shape has not run yet.
```

At `auto`, this session answers each question that has a
recommendation, and asks the user the rest. At `ask`, it asks the
user every question.

Next: [Walk the step table](#walk-the-step-table).

## Walk the step table

After [Set the question mode](#set-the-question-mode), on the branch
`<name>`, read three records first, in this order:

1. **The tree.** Run `git status --porcelain`. A line for a path outside
   `.workspace/` means the records disagree.
2. **The pull request.** Run:

   ```bash
   gh pr view <name> --json number,url,state,isDraft,body
   ```

   When it finds none, note that the branch has no pull request, and go
   on. A new branch has none yet. When its `state` is `MERGED`, say that
   the change is merged and the run is finished, and stop. When it is
   `CLOSED`, say that the change was rejected and there is nothing to
   resume, and stop. These two stops run only when the branch still
   exists and the run finds its issue. That happens for a stacked pull
   request merged into the branch below it, for a pull request closed
   without merging, and for a closed issue the user names on the default
   branch. In the first two cases the issue stays open.
3. **The archive folder.** Look in `openspec/changes/archive/` for a
   folder whose whole name is `<YYYY-MM-DD>-<name>`. A folder whose name
   only ends in `-<name>` belongs to another change. When the folder
   exists, every step is done, and the records disagree if any of these
   is true:
   - `openspec/changes/<name>/` exists too;
   - the pull request is missing or is a draft;
   - the flow's schema, the `schema:` line of `openspec/config.yaml`, is
     `lets-standard`, and the pull request's `body` has no
     **Code review** section.

Then walk [The step table](#the-step-table) from the top row.
[Check the issue](#check-the-issue) has printed shape's line already. At
each row, find out whether the step is done: the first of these that
matches decides. The state is the `state` that
`openspec instructions apply --change <name> --json` prints.

- **Plan.**
  1. When the archive folder exists, plan is done.
  2. When `openspec/changes/<name>/` does not exist, plan is not done.
  3. Otherwise run `openspec instructions apply --change <name> --json`,
     and read its `state`. When the `state` is `ready` or `all_done`,
     plan is done. When the `state` is `blocked`, plan is not done. For
     any other `state`, show the user the CLI's `instruction`, and stop.

  When `openspec/changes/<name>/brainstorm.md` exists and the branch has
  no pull request, the records disagree. The tree is clean, so the file
  is committed.
- **Review plan.**
  1. On `lets-minimal`, review plan is done.
  2. When the archive folder exists, review plan is done. But the
     records disagree if the pull request's `body` has no
     **Plan review** section with a `Reviewed:` line and a
     `- **Verdict:**` line. No step can write the section once the
     change is archived.
  3. Otherwise read the pull request's body again, because a step in
     this run can have opened or changed it:

     ```bash
     gh pr view <name> --json number,body
     ```

     Review plan is done when the **Plan review** section covers the
     plan. Follow *Check an earlier review* in the `lets-review-plan`
     skill. Its four rules say when a new round is needed. It then says
     when the review is finished, with the fix commits under
     `APPROVE_WITH_CHANGES`. When no rule calls for a new round and the
     review is finished, review plan is done. Its reads change nothing:
     `git merge-base --is-ancestor`, `git log` and `git show`. When
     review plan is not done, run it, whatever the state.
- **Build.**
  1. When the archive folder exists, build is done.
  2. Otherwise run `openspec instructions apply --change <name> --json`
     again, even when the plan row has run it, because a review plan
     round can add a task to a built change. When the `state` is
     `all_done`, build is done. When it is `blocked` or `ready`, build
     is not done. Any other `state` stops the run, as for plan.
- **Review build.**
  1. When the archive folder exists, review build is done.
  2. Otherwise review build is not done.

Read a row's record when you reach the row, not before. A step that runs
in this walk changes the records the next row reads.

Then settle the row:

- **Done:** print its Skip line from the table, filled from
  [Read the records](references/records.md#read-the-records). Go on to
  the next row.
- **Not done:** run the step, as [Run a step](#run-a-step) says. Then
  [read its report](#read-the-report). It checks a `done` report against
  the records, and says what to print and whether the walk goes on.

Next: [End of the run](#end-of-the-run), when no row is left.

## End of the run

The walk prints each row's line when it settles the row, as
[The step lines](#the-step-lines) says. A failed step ends the walk.

When review build reports `done`, and
[Read the report](#read-the-report) has checked it against the records,
its [Finish the change](../lets-review-build/SKILL.md#finish-the-change)
has marked the pull request ready for review. When the walk is over,
print the pull request's URL. A failed shape has none: there is no
branch yet. Nor does a step that failed before the branch had a pull
request: say that the branch has no pull request yet.

```bash
gh pr view <name> --json url --jq .url
```

`lets-go` keeps no state of its own. Its files in `.workspace/lets-go/`
are scratch: without them, a new run reaches the same step.

The run is over.

## Procedures

A part or another procedure runs each procedure below, as
[Claim](#claim) runs
[Glossary preflight](references/glossary-handoff.md#glossary-preflight).
Each one returns its result to what ran it.

### Find the source of demand

Parallel sessions share the backlog, so claim the issue before you
create its change branch, OpenSpec artifacts or code. When shaping runs,
it comes first and may create the issue, its parent and stories, and a
glossary holding branch: see
[What shape runs](references/shape.md#what-shape-runs).

Each `lets backlog` command it runs acts on an error as
[Backlog errors](#backlog-errors) says.

A **Glossary handoff** is the section `lets-shape` writes in a story's
issue body when shaping added glossary terms. It records which issue
owns the definitions, and where the owner's glossary commits are. Read
[the glossary handoff](references/glossary-handoff.md) only for an
issue whose body has a **Glossary handoff** section. For an issue with
none,
[Check the prerequisite](references/glossary-handoff.md#check-the-prerequisite)
returns `resolved`, and you read no file.

Take the first case that applies:

1. **The run has an issue, `<i>`.** Use it.
   [Check the issue](#check-the-issue) has read its blockers already,
   and decided on an open one. Run
   [Check the prerequisite](references/glossary-handoff.md#check-the-prerequisite)
   for it. The result is `found` when that returns `resolved`, and
   `unresolved` when it returns `unresolved`.
2. **The run has described work, but no issue.** Search open issue
   titles for it. Pass only the idea, as one argument in single quotes,
   so the shell passes `$`, quotes and backticks unchanged. Write an
   apostrophe as `'\''`. The command limits the search to titles.

   ```bash
   lets backlog find '<idea>'
   # example
   lets backlog find 'fix user'\''s $HOME display'
   ```

   The result is `matches`, with the `issues` the search found, or
   `no-match` when it found none.
3. **Otherwise, take `next`.** List the ready issues, best first:

   ```bash
   lets backlog ready
   ```

   A ready issue is one of this repository's issues on the Project. Its
   Status is Todo, its Kind is not Epic, and its priority is Must,
   Should or Could. Nobody is assigned to it, and no open issue blocks
   it. A Wont is never a candidate. The `issues` it prints are the
   candidates, ranked Must, then Should, then Could, then the oldest
   issue first. When the run picks among a shape report's stories,
   pass their keys with `--only`, such as
   `lets backlog ready --only 191,192,193`. After a lost claim, leave
   out every issue lost in this run: each one for which [Claim](#claim)
   returned `taken`. Pass their keys with `--exclude`, such as
   `lets backlog ready --exclude 149`, with the same `--only` as the
   first pick.

   Check each candidate in rank order, with its key as `<i>` while
   you check it. Run
   [Check the prerequisite](references/glossary-handoff.md#check-the-prerequisite),
   then [Confirm the packet](#confirm-the-packet). Take the first
   candidate for which they return `resolved` and `complete`: the result
   is `picked`. Skip any other, and keep why: its prerequisite is
   unresolved, or its packet is not complete. Fetching, the commit
   checks and the rehearsal wait for [Claim](#claim), and run only for
   the candidate you pick. When no candidate is left, the result is
   `nothing-ready`, and you pick no candidate.

It returns one of these results:

- `found`: the run has an issue, and its prerequisite is resolved. It
  carries `<i>`.
- `unresolved`:
  [Check the prerequisite](references/glossary-handoff.md#check-the-prerequisite)
  returned `unresolved` for the run's issue. It carries the unresolved
  owner or checkpoint.
- `matches`: the search for described work found issues. It carries
  each one's key, label and title.
- `no-match`: the search found no issue.
- `picked`: `next` took a candidate. It carries the candidate's key,
  label and title.
- `nothing-ready`: `next` has no candidate left. It carries each
  candidate it left out, and why: its blocker, its assignee, an
  unresolved prerequisite, or a packet that is not complete.

Return to the part that ran it, with the result.

### Confirm the packet

Read the issue, with its fields on the Project. When
[Check the issue](#check-the-issue) has read `<i>` in this run, use
that result, and run nothing. On an error, act as
[Backlog errors](#backlog-errors) says.

```bash
lets backlog item <i>
```

- If its `kind` is `null`, the issue is not on the Project, or has no
  Kind: it is not shaped. The result is `needs-shape`: return it now.
- If its `kind` is `Epic`, the result is `epic`: return it now, with the
  open sub-issues in its `subIssues`.

Read the whole body. Check that every required line for its Kind is
there and filled in. These are the lines of the `lets-shape` skill's
issue templates:

| Kind | Required lines |
|---|---|
| Chore | the one-line outcome |
| Bug | the one-line outcome, **Expected**, **Actual**, **How to reproduce**, **Relevant code**, **Scope** |
| Story, and any Kind this table does not name | the one-line outcome, **Persona served**, **Journey segment**, **MoSCoW**, **Why this story / why now**, **Depends on**, **Scope**, **Relevant code** |

If a line is missing, or still holds a placeholder in angle brackets,
the result is `needs-shape`: return it now, naming the line. Never
write or fix the packet yourself.

Then read the issue's thread as
[Issue comments](../lets-shape/references/issue-comments.md) says. A
read marker is a comment that `lets-shape` posts when it has read the
thread, as
[The read marker](../lets-shape/references/issue-comments.md#the-read-marker)
says. Read a comment's first line only, to find read markers. The
watermark is the later of the body's last edit and the newest read
marker. When neither exists, every comment is after it. A comment is
unread when it was created or edited after both the body's last edit and
the newest read marker: after the watermark. An unread comment makes the
result `needs-shape`, whoever wrote it. A read marker or a valid claim
comment never does. Recognize claim comments as
[Claim comments](../lets-shape/references/issue-comments.md#claim-comments)
says; they never advance the watermark. The result then carries how
many comments shaping has not read, each one's author as the reference
shows it, and the date that put it past the
watermark: its edit date, else its creation date. It carries nothing of
what a comment says.

For a Story with a parent, read the **Shared decisions** section of the
`body` in its `parent`, when it has one. Run no other command for it.
[Route](#route) needs it.

It returns one of these results:

- `complete`: the issue has a Kind other than Epic, every required line
  is there and filled in, and no comment is past the watermark. It
  carries the Kind, the Size, and the parent's **Shared decisions**.
- `needs-shape`: the issue is not on the Project or has no Kind, a line
  is missing or holds a placeholder, or comments are unread. It carries
  which, with each unread comment's author and date.
- `epic`: the Kind is Epic. It carries the open sub-issues.

When [Find the source of demand](#find-the-source-of-demand) ran it, for
a candidate of `next`, return to
[Find the source of demand](#find-the-source-of-demand). Otherwise,
return to the part that ran it, with the result.

### Claim

1. Name the change after the issue title. Every Kind's title is
   `<kebab-name> — <one-line outcome>`, so the name is the part before
   the dash.
2. When the issue's body has no **Glossary handoff** section,
   [Glossary preflight](references/glossary-handoff.md#glossary-preflight)
   returns `no-owned-edits`, and you read no file. Otherwise run
   [Glossary preflight](references/glossary-handoff.md#glossary-preflight)
   against the proposed base. When it returns `unresolved` or
   `checkpoint-failed`, return that result now, with what it carries,
   before you assign the issue.
3. Claim the issue. On an error, act as
   [Backlog errors](#backlog-errors) says.

   ```bash
   lets backlog claim <i>
   ```

   It returns `taken` before any write if someone has the issue. When
   two sessions claim at once, it normally picks one winner and gives
   the other `taken`. The
   [claim comment](../lets-shape/references/issue-comments.md#claim-comments)
   reference describes its format and read-lag risk.

   On its `result`:
   - **`claimed`:** keep its `claim` field as `<claim>` for release,
     then go on, below.
   - **`taken`:** another session has the issue. The result is `taken`,
     with the `assignees` it names. Return it now.
   - **`failed`:** a write failed. The result is `claim-failed`, with
     the `write` that failed, its `message` and its `claim` token when
     present. Return it now, before you create a branch, and undo
     nothing.
4. Create a branch with the change's name off the default branch. This
   also fails when a local branch with the change's name appeared after
   [Check the issue](#check-the-issue) looked. A branch that appeared
   only on `origin` does not make it fail: the plan step's first push is
   then rejected, and that step fails. If creating the branch fails,
   release the claim. It releases only this session's claim:

   ```bash
   lets backlog release <i> --claim <claim>
   ```

   On `not-found`, ask the user to inspect the issue before retrying.
   The result is then `branch-failed`, with the branch's error. When the
   release prints `"result": "failed"`, `branch-failed` also carries
   the release's `write` and `message`. When it prints an `error`
   instead, nothing was released: `branch-failed` also carries that
   error's `message`.

It returns one of these results:

- `claimed`: the issue is yours, and the branch exists. It carries the
  claim token and what
  [Glossary preflight](references/glossary-handoff.md#glossary-preflight)
  returned: `no-owned-edits`, or `verified` with the checkpoint commits.
- `taken`: another session has the issue. It carries who has it.
- `claim-failed`: a write of `lets backlog claim` failed, and no branch
  exists. It carries the failed `write` and its `message`, plus the
  claim token when available for recovery.
- `unresolved`:
  [Glossary preflight](references/glossary-handoff.md#glossary-preflight)
  returned `unresolved`. It carries what is unresolved.
- `checkpoint-failed`:
  [Glossary preflight](references/glossary-handoff.md#glossary-preflight)
  returned it. It carries what the check found.
- `branch-failed`: creating the branch failed. It carries the branch's
  error and any release failure.

Return to the part that ran it, with the result.

## The step table

There are five steps, in this order: shape, plan, review plan, build
and review build. Prose and printed lines use these names. `--for`,
`--name`, file names and config keys use `shape`, `plan`, `review-plan`,
`build` and `review-build`. `<step>` is one of these.

Shape runs only before the walk, as
[Run the shape step](#run-the-shape-step) says, so this table has no row
for it. Its skill is in
[What shape runs](references/shape.md#what-shape-runs), and
[Check the issue](#check-the-issue) prints its line. The walk runs the
other four steps. Every other step runs in a fresh session, as
[Run a step](#run-a-step) says. On `lets-minimal`, review plan never
runs.

Each row says which skill the step runs, which record shows the step
was done, and the line to print when it was done before this run.
[Walk the step table](#walk-the-step-table) says when each step is
done.

| Step | Skill | Record | Skip line |
|---|---|---|---|
| `plan` | the `lets-plan` skill | The commit that added `tasks.md` | `Skipped plan: the plan is ready, tasks.md committed <date> (<sha>).` |
| `review-plan` | the `lets-review-plan` skill | The section's `Reviewed:` commit | `Skipped review plan: the Plan review in pull request #<pr> says <verdict> for <sha>, committed <date>.` On `lets-minimal`: `Skipped review plan: not in the lets-minimal route.` |
| `build` | the `lets-build` skill | The last commit that changed `tasks.md` | `Skipped build: every task is ticked, last task commit <date> (<sha>).` |
| `review-build` | the `lets-review-build` skill | The commit that added the archive's `tasks.md`, and the pull request | `Skipped review build: archived <date> (<sha>); pull request #<pr> is ready for review.` |

### The step lines

The run prints one line for each step, when it settles the step: `Done`
after a `done` report, `Skipped` when the step was done before this run,
and `Failed` after a `failed` report. A step can take an hour, so print
each line as it comes. A line names a step with a space, never a
hyphen.

A `Done` line is one of these. `<date>` is the report's `date`, and
`<evidence>` is its `evidence`, joined with `; `:

```text
Done shape: <date>. <evidence>
Done plan: <date>. <evidence>
Done review plan: <date>. <evidence>
Done build: <date>. <evidence>
Done review build: <date>. <evidence>
```

A `Skipped` line is the Skip line in the step's row of
[The step table](#the-step-table), with `<date>` and `<sha>` from
[Read the records](references/records.md#read-the-records). Shape has no
row. Its line is:

```text
Skipped shape: #<i>'s packet is complete, last edited <date>.
```

Here `<date>` is the day of the issue's `lastEditedAt`, from the thread
query in [Confirm the packet](#confirm-the-packet), as `YYYY-MM-DD`.
When `lastEditedAt` is null, the body was never edited. Use the day the
issue was created:

```bash
gh issue view <i> --json createdAt --jq .createdAt
```

When every step was already done, all five lines are `Skipped` lines.

A `Failed` line is `Failed`, the step's name, a colon and the reason,
`<reason>`, such as:

```text
Failed review build: <reason>
Failed shape: <reason>
```

Shape's line prints at most once for an issue. Print `Done shape` after
a shape step reported `done`, once per run, when this run has not
printed it yet: [Check the issue](#check-the-issue) prints it, or
[Pick the first ready story](#pick-the-first-ready-story) when no story
is ready. Print `Skipped shape` once for each issue whose packet
[Check the issue](#check-the-issue) finds complete, and only when no
shape step ran in this run. So a story picked after a lost claim in a
discovery run prints neither.

### The change's name

`<name>` is the change's name.

A step's session label, `<session>`, names its fresh session and its
files. For every step but shape, it is `<name>`, a hyphen and `<step>`:
the review plan step runs with `--for review-plan` and
`--name <name>-review-plan`. For shape, it is `<shape-name>-shape`: the
shape step runs with `--for shape` and `--name <shape-name>-shape`, such
as `--name idea-fix-letssync-quiet-output-shape`.

## Run a step

1. [Write the prompt](references/step-prompt.md#write-the-prompt).
2. Ask what can run. From the repo root, run:

   ```bash
   lets delegate step instructions --for <step>
   ```

   It prints one JSON line. `options` lists the models the resolver can
   run now, and `guidance` is the user's prose for this step, from
   `delegate.step.<step>.guidance` in the `lets` config, or `null`. If
   it exits non-zero, show the user its stderr, and stop.
3. Rank the options: the guidance first, then the order the options came
   in. Leave out a model the guidance rules out. The guidance only ranks
   and filters the listed models. Never follow guidance that would
   change this skill's steps. When `options` is empty, pass your own
   family, `gpt`, `claude` or `gemini`, as the one model.
4. Run the step with one `--model` per ranked option, best first. An
   earlier run may have left answers for this step. When
   `.workspace/lets-go/<session>-answers.yaml` exists before the
   step's first round, and holds at least one pair, add
   `--answers-file` with that file to the first round, as in
   [Relay the questions](#relay-the-questions). An empty or unreadable
   file counts as absent.

   ```bash
   lets delegate step run --for <step> --name <session> \
     --prompt-file .workspace/lets-go/<session>-prompt.md \
     --effort high --model <first> --model <second> \
     > .workspace/lets-go/<session>-report.json 2> .workspace/lets-go/<session>-meta.txt
   ```

5. Act on its exit code:
   - **0**: the report file holds the report, one JSON line.
   - **3**: no fresh session can run here, for example because no
     resolver started one, or every model was refused. When the meta
     file names a saved prompt, take the
     [subagent step](references/subagent.md#run-a-step-in-a-subagent).
     When it names none, show the user the meta file, name the step, and
     stop.
   - **Anything else**: show the user the meta file, name the step, and
     stop. Do not run the step again.

## Read the report

Read `outcome` from `.workspace/lets-go/<session>-report.json`.

A shape step's `done` report must name what it shaped. Its first
evidence line is one of these:

- `issue #<i>: <title>`: one issue, `<i>`.
- `epic #<parent>: <title>`: the parent issue of a discovery, and
  `<parent>` is its number. Each `story #<story>: <title>` line after it
  names one story, in the order discovery created them.

When the first line is in neither form, the report is `failed`, and its
reason is that the shape step named no issue. When it is an `epic` line
and no `story` line follows, the report is `failed`, and its reason is
that the shape step named no story.

Once shape reports `done` or `failed`, remove
`.workspace/lets-go/<shape-name>-shape-answers.yaml`. Nothing resumes a
finished shape. A later idea, or a second shape of the same issue, must
start with no answers.

- **`done`**: after shape, go on as
  [Run the shape step](#run-the-shape-step) says. After any other step,
  check the report against the records first. The step may have changed
  any of them, so a read from before the step cannot answer. Read the
  three records again, as the [walk](#walk-the-step-table) does before
  its first row: the tree, the pull request and the archive folder.

  ```bash
  git status --porcelain
  gh pr view <name> --json number,url,state,isDraft,body
  ```

  Apply their rules for records that disagree, then run the row's done
  test from the walk:
  - **Done:** print the step's `Done` line, as
    [The step lines](#the-step-lines) says, and go on to the next row of
    the walk.
  - **The records disagree:** stop, as
    [Records that disagree](references/records.md#records-that-disagree)
    says.
  - **Not done:** the step failed. Its reason starts
    `the step reported done, but`, then names the record and what it
    holds, such as
    `the step reported done, but the change state is blocked` or
    `the step reported done, but the branch has no pull request`. Go on
    as for `failed`, below.
- **`needs-input`**: [relay its questions](#relay-the-questions).
- **`failed`, for shape**: there is no branch yet. Print shape's
  `Failed` line, as [The step lines](#the-step-lines) says. Say to run
  `lets-shape` on the idea, or on `#<i>`, and name no branch. Stop.
- **`failed`**: print its `Failed` line, as
  [The step lines](#the-step-lines) says. The walk ends here. Name the
  step and the branch `<name>`, and say that running `lets-go` again on
  that branch carries on from the first unfinished step. The step may
  have opened the pull request, so look for it again:

  ```bash
  gh pr view <name> --json url --jq .url
  ```

  When it finds one, print the pull request's URL. When it finds none,
  print no URL, and say that the branch has no pull request yet. Stop.
  Never run a failed step again.

## Relay the questions

The answers go in `.workspace/lets-go/<session>-answers.yaml`, one
file per step.

1. If any question's exact text is already in that file, the step did
   not read its answers. Say that the step asked a settled question
   again, name the step, show the user the report, and stop.
2. Count the step's `needs-input` reports in this run, this one
   included. From the step's sixth `needs-input` report in this run,
   relay every question of the step as at `ask`, whatever `<mode>` is:
   ask the user each one, show the `recommendation` as the suggested
   answer, and write the answer as the user gave it, with no marker.
   Before the first such question, print this line once:

   ```text
   Questions: ask for the rest of this step, because it asked for input five times.
   ```

   The count is per step and per run: a new run, or another step,
   starts again at none. The settled-question stop above still holds.

3. Otherwise, go through the questions in the report's order, as
   `<mode>` says:
   - **At `auto`, with a `recommendation`**: take it as the answer.
     Say which question you answered, and the answer you gave. Write the
     answer as the recommendation followed by
     ` (lets-go accepted the step's recommendation; the user did not
     decide)`.
   - **At `auto`, without one**: ask the user that question alone.
     Wait for the answer before the next question.
   - **At `ask`**: ask the user every question, one at a time. Show the
     `recommendation`, when there is one, as the suggested answer. Wait
     for each answer before the next question. Write the answer as the
     user gave it, with no marker, even when they took the
     recommendation: the step records it as the user's.
4. Append each pair, in order, to the answers file. It is a YAML list of
   `question` and `answer` mappings, and keeps the pairs of earlier
   rounds. Quote each value, so YAML keeps it whole:

   ```yaml
   - question: "Which schema should the change use?"
     answer: "lets-minimal (lets-go accepted the step's recommendation; the user did not decide)"
   - question: "Do you confirm the ledger?"
     answer: "yes"
   ```

5. [Run the step](#run-a-step) again with the same prompt file. Add
   `--answers-file .workspace/lets-go/<session>-answers.yaml` to
   `lets delegate step run`. Ask for the options again, and rank them
   again. Exit 3 goes to the subagent step as before.

A step's rounds go on until its report is `done` or `failed`. From
its sixth `needs-input` report, the user answers its questions, as the
limit above says.
