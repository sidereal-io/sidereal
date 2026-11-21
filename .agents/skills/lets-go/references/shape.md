# Shape

Read this file when the run starts from an idea, runs the shape step,
or commits the discovery map.

## Start from an idea

1. Look for a discovery of this idea that an earlier run started. Use
   `lets-shape`'s own query:

   ```bash
   gh issue list --state open --search '"Discovery status: in-progress" in:body' --json number,title,body
   ```

   When a parent's `Input:` line holds the idea word for word, say that
   a discovery of this idea is in progress at `#<parent>`. Skip the
   search below, and
   [run the shape step](../SKILL.md#run-the-shape-step) for the idea:
   its round resumes that parent.
2. Run the second case of
   [Find the source of demand](../SKILL.md#find-the-source-of-demand):
   the run has described work, but no issue. It searches open issue
   titles for the idea's words. On its result:
   - **`matches`:** show the user the matches it carries, and ask
     whether one is the same work. Ask in either mode: a wrong yes
     builds someone else's issue.
   - **`no-match`:**
     [run the shape step](../SKILL.md#run-the-shape-step) for the idea.
3. On a yes, that issue is `<i>`. Go on from the blocker read in
   [Check the issue](../SKILL.md#check-the-issue).
4. On a no, [run the shape step](../SKILL.md#run-the-shape-step) for the
   idea.

## Run the shape step

The shape step turns an idea, or an issue whose packet is not complete,
into one shaped issue. When the work is too large for one issue, it runs
discovery instead, which files a parent issue and its stories. It runs
on the default branch, before any branch or change exists, and at most
once per run. [Check the issue](../SKILL.md#check-the-issue) decides
when it runs.

1. Check the discovery maps:

   ```bash
   git status --porcelain -uall -- openspec/discovery.md openspec/discovery
   ```

   It names each changed map, new files included, and works when the
   `openspec/discovery/` folder does not exist. When it lists a path,
   look for this input's discovery: an open
   parent issue that says `Discovery status: in-progress`, and whose
   `Input:` line names this input, the idea's words or `#<i>`. For an
   idea, use what [Start from an idea](#start-from-an-idea) found. For
   an issue, run the query in [Start from an idea](#start-from-an-idea).
   - **There is one:** the edits are that discovery's confirmed phases.
     Go on: the step resumes it, and its map commit takes them.
   - **There is none:** say that the maps have uncommitted edits, name
     each path it lists, and say to commit or discard them. Stop, and
     run no step. If the run went on, a discovery's commit could sweep
     those edits in.
2. Set `<mode>` and print its line, as
   [Set the question mode](../SKILL.md#set-the-question-mode)
   says for a question mode before a shape step. Then go on here, with
   the next step.
3. For an idea, check that the answers file is this idea's. Two ideas
   can share their first five words, and so their `<shape-name>`. When
   `.workspace/lets-go/<shape-name>-shape-prompt.md` exists and holds
   another idea, an earlier run stopped before it finished, on that
   other idea. In that case, remove
   `.workspace/lets-go/<shape-name>-shape-answers.yaml` before you write
   the prompt.
4. Run the shape step, as [Run a step](../SKILL.md#run-a-step) says.
5. When it reports `done`,
   [Read the report](../SKILL.md#read-the-report) says what it shaped:
   - **One issue:** that issue is `<i>`. Go on from the blocker read in
     [Check the issue](../SKILL.md#check-the-issue) with it, as for an
     issue number.
   - **A parent and its stories:**
     [pick the first ready story](../SKILL.md#pick-the-first-ready-story).

## Pick the first ready story

Discovery filed a parent issue, `#<parent>`, and its stories. Build one
of them in this run.

1. Run the third case, `next`, of
   [Find the source of demand](../SKILL.md#find-the-source-of-demand),
   with one narrowing: the candidates are the stories the report named,
   and no other issue. Pass their keys to `lets backlog ready` with
   `--only`, as the section says. Rank as it does: Must, then Should,
   then Could, then the oldest issue first. Run
   [Check the prerequisite](glossary-handoff.md#check-the-prerequisite)
   and [Confirm the packet](../SKILL.md#confirm-the-packet) on each
   candidate, in rank order. A ready story is one that `next` would
   take.
2. On its result:
   - **`picked`:** print:

     ```text
     Picked #<i>: <title>, the first ready story under #<parent>.
     ```

     That story is `<i>`. Go on from the blocker read in
     [Check the issue](../SKILL.md#check-the-issue) with it, as for an
     issue number.
   - **`nothing-ready`:** print the `Done shape` line, as
     [The step lines](../SKILL.md#the-step-lines) says. Say
     that no story under `#<parent>` is
     ready. Name each story, and why it was left out, as the result
     carries it: for example, its blocker, its assignee, an unresolved
     prerequisite, or a packet that is not complete. For a story that
     `lets backlog ready` did not list, read why with
     `lets backlog item <story>`: the `assignees` it names, an open
     issue in its `blockedBy`, or its `status`, `kind` or `priority`.
     On an error, act as [Backlog errors](../SKILL.md#backlog-errors)
     says. Say to run `lets-go` with a story's number once it is ready.
     Then stop. Claim nothing, and create no branch.

A run that stops after discovery and before [Claim](../SKILL.md#claim),
for any reason, leaves the maps' edits on the default branch. When the
status command in [Commit the discovery map](#commit-the-discovery-map)
lists a path, name each one, and say that they hold the discovery's
edits, uncommitted on the default branch. While the parent still says
`Discovery status: in-progress`, say to leave the maps as they are:
running `lets-go` again with the same idea, or `#<i>`, resumes the
discovery and keeps its edits. Otherwise, say to commit or discard them
before the next run, because the next run stops on them.

## Commit the discovery map

Discovery edits the discovery maps on the default branch:
`openspec/discovery.md`, and in a repository with several products the
files under `openspec/discovery/`. The new branch carries the edits with
it. When a shape step ran in this run, list the changed maps on the new
branch, before [Walk the step table](../SKILL.md#walk-the-step-table)
reads the tree:

```bash
git status --porcelain -uall -- openspec/discovery.md openspec/discovery
```

Each line is two status letters, a space, then a path. When it lists no
path, make no commit. Otherwise take each path, new files included, as
`<paths>`, and commit them:

```bash
git add -- <paths>
git commit -m "docs: the discovery map from #<parent>" -- <paths>
```

Never pass the `openspec/discovery` folder itself to `git add` or
`git commit`: both fail on a path that matches no file.
`<parent>` is the parent's number, or the story's when the parent
became the one story. Commit no other path. This is the one commit
`lets-go` makes itself, beside the glossary import.

Return to the part that ran it.

## The shape step's name

`<name>` is the change's name. Shape runs before there is one, so it
has a name of its own, `<shape-name>`, from what it shapes:

- **An issue:** `issue-<i>`, such as `issue-180`.
- **An idea:** `idea-<slug>`. To make `<slug>`, split the idea on white
  space. Lower-case each word, and keep only its letters and digits.
  Drop a word left empty. Join the first five words with hyphens, or all
  of them when there are fewer. `fix lets-sync — quiet output` becomes
  `idea-fix-letssync-quiet-output`. The same idea typed again gets the
  same name, so it finds its answers file.
  [Read the request](../SKILL.md#read-the-request) has already stopped
  an idea with no letter or digit, so `<slug>` is never empty, and no
  shape step is named `idea-`.

## What shape runs

The shape step runs `lets-shape`. It shapes one issue, or runs discovery
when the work is too large for one issue. A fresh round must find the
discovery an earlier round started, so the text lists where to start, in
order.

```text
Run the `lets-shape` skill. Start at the first of these that matches:

1. An open parent issue says `Discovery status: in-progress`, and its
   `Input:` line names this input: the idea's words, or #<i>. Run its
   *Resume mode* on that parent, and start no second discovery. The
   phases that the `## Answers` section confirmed are settled. So skip
   the restatement in *Resume mode* and its confirmation, and go straight
   to the parent's recorded next phase.
2. An open parent issue's `Input:` line names this input, it has no
   status line, and the `## Answers` section confirmed its Phase 5 story
   table. Finalize has started on that parent: Finalize removes the
   status line first, then asks its own questions. Finish Finalize on
   that parent, from where the answers leave off, including the glossary
   handoff. Start no second discovery.
3. The `## Answers` section answers the discovery question with yes. Run
   its discovery from Ingest, with the idea or #<i> as its input.
4. Otherwise, run its *Shape one issue* to its end, including its second
   comment fetch, its reply and its glossary handoff. For an issue: shape
   #<i>. For an idea: lets-go has searched for it, and the user confirmed
   that no open issue is the same work. Skip the search in
   **Find or create the issue**, and create the issue. The packet
   confirmation in **Confirm, then write** is a confirmation question:
   give it no recommendation.

The work is too large for one issue when it is Size L or XL, or when the
input holds a large idea. Then do not fail, and do not start discovery
yourself. End the round with a needs-input report that holds this
question, with the recommendation yes:

The work is Size <L or XL>, too large for one issue. Run discovery on it?

If the answer is anything but yes, end with a failed report. Its reason
says that the work is too large for one issue and discovery was
declined.

Discovery runs every phase from Ingest to Finalize, including the
glossary handoff. Write the idea's words, or #<i>, in the parent's
`Input:` line, so a later round can find the parent. Each phase's
confirmation, and the story table in Phase 5, is a confirmation
question: give it no recommendation. The question about which product
an idea belongs to, and the question that moves the map for a second
product, are confirmation questions too: give them no recommendation.
When the glossary handoff asks who carries the edits, recommend the
first story in dependency order.

Create no change and no change branch. The parent issue, the stories
and a glossary holding branch that `lets-shape` makes are allowed. End on
the branch <default>, with a clean tree, except the discovery maps,
`openspec/discovery.md` and the files under `openspec/discovery/`, when
discovery ran.

A done report's first evidence line is `issue #<i>: <title>` for one
issue. When discovery filed stories, it is `epic #<parent>: <title>`,
then one `story #<story>: <title>` line per story, in the order you
created them.
When discovery found that the whole run fits one story, the parent became
that story: use the `issue` form.
```
