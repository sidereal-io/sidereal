# Story run

Read this file only in a story run's child session: a `lets-go`
session whose request starts with
`You run a story run of a lets-go epic run:`.

An epic run has picked the story, claimed it, routed it and checked its
branch out in a git worktree of its own, which the story prompt names.
That worktree is this session's repository root: run every command
from it, and write no file outside it. This session walks the story's
steps on that branch, and ends with one step report, which the epic
run reads. This session is not an
epic run: where a part of `SKILL.md` says what happens in an epic run,
that is for the epic run's own session, not for this one.

## Take up the story run

[Read the request](../SKILL.md#read-the-request) sends the run here,
after [Read the settings](../SKILL.md#read-the-settings) has found the
default branch and the current branch, `<branch>`.

1. Take these from the story prompt: `<i>`, `<name>` and `<parent>`
   from its first line; `<issue-url>`, `<schema>` and `<route line>`
   from its second line; `<mode>` from its `## Questions` section; and
   its `## Stack` section, when it has one, with the `<blocker>`,
   `<blocker-branch>` and `<blocker-pr>` it names. A story prompt with
   no `## Stack` section is for a story that is not stacked.
2. Check the branch. When `<branch>`, from `git branch --show-current`,
   is not `<name>`, run no step, and end the session with this report:

   ```step-report
   outcome: failed
   reason: "stopped: the story run is on <branch>, not <name>"
   ```

3. Run none of [Check the issue](../SKILL.md#check-the-issue),
   [Route](../SKILL.md#route),
   [Enter the branch](../SKILL.md#enter-the-branch),
   [Claim](../SKILL.md#claim) and
   [Set the question mode](../SKILL.md#set-the-question-mode): the epic
   run ran them for the story. Claim nothing, and stay on `<name>`.
4. Walk [The step table](../SKILL.md#the-step-table) from its top row
   on the branch `<name>`, as
   [Walk the step table](../SKILL.md#walk-the-step-table) says. Run
   each step that is not done as [Run a step](../SKILL.md#run-a-step)
   says, with the
   [subagent step](subagent.md#run-a-step-in-a-subagent) on that step's
   exit 3. Each step's prompt holds `<mode>` from the story prompt, and
   the story prompt's `## Stack` section when it has one, as
   [Write the prompt](step-prompt.md#write-the-prompt) says.

   When the story prompt ends with an `## Answers` section, first
   [Give the answers to the step](#give-the-answers-to-the-step) at the
   first row that is not done. When a step reports `needs-input`,
   [Pass the questions up](#pass-the-questions-up) instead of relaying
   them. When the walk ends, or stops, go on to
   [End the story run](#end-the-story-run).

## Pass the questions up

When a step reports `needs-input`, this session answers none of its
questions, at either question mode: the epic run applies `<mode>`, and
asks the user. Take these actions in place of
[Relay the questions](../SKILL.md#relay-the-questions):

1. Apply the settled-question check of
   [Relay the questions](../SKILL.md#relay-the-questions). When any
   question's exact text is already in
   `.workspace/lets-go/<session>-answers.yaml`, the step did not read
   its answers. Pass no question up, and end the session with a
   `failed` report whose reason is
   `stopped: the <step name> step asked a settled question again`.
2. Count the round. Append one line to
   `.workspace/lets-go/<session>-rounds.txt`, then count its lines:

   ```bash
   date -u +%Y-%m-%dT%H:%M:%SZ >> .workspace/lets-go/<session>-rounds.txt
   wc -l < .workspace/lets-go/<session>-rounds.txt
   ```

   The count is the step's rounds in this story run, this one included.
   The epic run cleared the file before the story's first child
   session, and leaves it alone between rounds.
3. End the session with a `needs-input` report that holds the step's
   questions, in the step's order, each with the step's
   `recommendation` when it has one. Copy each question and
   recommendation word for word: the answers come back keyed by the
   question's exact text. From the step's sixth round, when the count
   is 6 or more, give each question `ask: true` too, so that the epic
   run asks the user, whatever `<mode>` is:

   ```step-report
   outcome: needs-input
   questions:
     - question: "Which schema should the change use?"
       recommendation: "lets-minimal"
       ask: true
   ```

   Before the sixth round, give no question `ask`.

## Give the answers to the step

When the story prompt ends with an `## Answers` section, the epic run
has answered the questions of this story run's last `needs-input`
report. Each `- Q:` and `A:` pair is one question and its answer.
At the first row of the walk that is not done, give them to the step
that asked, in place of that row's **Not done** action in
[Walk the step table](../SKILL.md#walk-the-step-table):

1. Read `.workspace/lets-go/<session>-report.json`, the step's last
   report. Check it before you write anything. When it is not a
   `needs-input` report, or a pair matches none of its questions, write
   nothing, run no step, and end the session with this report:

   ```step-report
   outcome: failed
   reason: "stopped: the answers match no question the <step name> step asked"
   ```

2. Otherwise, append each pair of the `## Answers` section whose
   question is one of that report's questions to
   `.workspace/lets-go/<session>-answers.yaml`, in order, as
   [Relay the questions](../SKILL.md#relay-the-questions) writes pairs.
   Keep the pairs of earlier rounds.
3. Run the step again, as [Run a step](../SKILL.md#run-a-step) says,
   with the same prompt file and
   `--answers-file .workspace/lets-go/<session>-answers.yaml`. Ask for
   the options again, and rank them again. Then
   [read its report](../SKILL.md#read-the-report), which says whether
   the walk goes on.

## End the story run

This session ends with one step report, which the epic run reads, and
prints no pull request URL and no epic line in place of it. Run none of
[A stop in an epic run](epic-run.md#a-stop-in-an-epic-run): the epic
run settles the story from the report. The report is one of these:

- **Every row done:** the walk ended with no row left, even when every
  row was done before this session started and the walk printed only
  `Skipped` lines. Read the pull request's number and URL as `<pr>` and
  `<pr-url>`:

  ```bash
  gh pr view <name> --json number,url
  ```

  End with a `done` report whose first evidence line names the pull
  request:

  ```step-report
  outcome: done
  evidence:
    - "pull request #<pr>: <pr-url>"
  ```

- **A failed step:** [Read the report](../SKILL.md#read-the-report)
  ended the walk on a `failed` report, including a `done` report that
  its record does not bear out. Print the step's `Failed` line, then
  end with a `failed` report whose reason is `failed at <step name>: `
  followed by the step's reason. `<step name>` is the step's name in
  plain words, as its `Failed` line names it, such as `review build`.
  For a build step that failed because the tests fail on main:

  ```step-report
  outcome: failed
  reason: "failed at build: tests fail on main"
  ```

- **Any other stop of the walk:** such as records that disagree, a pull
  request that is merged or closed, a settled question asked again, a
  change state the walk does not know, or a `lets delegate step run`
  exit code that stops the walk. End with this report, where the reason
  is what the stop says:

  ```step-report
  outcome: failed
  reason: "stopped: <reason>"
  ```
