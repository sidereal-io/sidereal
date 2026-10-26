---
name: lets-shape
description: Break a large product requirements document (PRD), vision, issue, or product idea into a release plan for an OpenSpec project — personas, journey map, MoSCoW priorities, and thin vertical stories filed as GitHub issues on the repository's Project, which lets-build picks up one at a time. Use when a feature or PRD is too big for one OpenSpec change, when the user asks to split or prioritize work, when a new request must be fitted into an existing backlog, to shape a single bug, chore or story into a complete issue, or to resume or revise a discovery.
---

# Let's shape

Shape a large idea into a backlog. This is discovery: personas, a journey map, MoSCoW priorities (Must, Should, Could, Won't) and thin vertical stories. It also shapes one bug, chore or story on its own: see [Shape one issue](#shape-one-issue). Run every command below from the repo root.

**Discovery plans; it does not build.** Read files, search code and investigate the codebase. Never write application code, and never create a change. Discovery writes the map, issues, and agreed project-owned glossary definitions: no roadmap, release plan or changelog.

- `openspec/discovery.md` — the living map: personas and journey map ([templates/discovery-template.md](templates/discovery-template.md)). It outlives every run; see [The map](#the-map).
- GitHub issues — one **parent issue** per discovery run, and the stories ([templates/issue-template.md](templates/issue-template.md)).

Changes are created later, one at a time, by `lets-build`.

**The input is read-only.** A discovery run starts from an input: a PRD or vision file, a GitHub issue, pasted text, or just conversation. Read it. Never create, edit or restate it in a file. The run records what it decides in its parent issue: the confirmed scope, the priorities and the Won't calls.

**This is a stance with a light structure.** The phases below give the conversation a shape. They are not a script. Revisit an earlier phase when new information arrives, and confirm with the user before you move to the next.

## The Project

Kind, Priority, Size and Status are fields on the GitHub Project linked to this repository. Set them only with the `gh project item-edit` commands below. Kind is Epic, Story, Bug or Chore. Priority is Must, Should, Could or Wont. Size is XS, S, M, L or XL.

Find the Project first:

```bash
gh api graphql -F owner='{owner}' -F name='{repo}' -f query='
query($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    nameWithOwner
    projectsV2(first: 100) { nodes { number title url closed owner { ... on User { login } ... on Organization { login } } } }
  }
  repositoryOwner(login: $owner) {
    ... on ProjectV2Owner {
      projectsV2(first: 100) { nodes { number title url closed owner { ... on User { login } ... on Organization { login } } repositories(first: 100) { nodes { nameWithOwner } } } }
    }
  }
}' --jq '.data.repository.nameWithOwner as $repo
  | {repo: $repo, projects: ([.data.repository.projectsV2.nodes[], (.data.repositoryOwner.projectsV2.nodes[] | select(any(.repositories.nodes[]; .nameWithOwner == $repo)))]
      | map(select(.closed | not) | {number, title, url, owner: .owner.login}) | unique_by(.owner, .number))}'
```

The query lists the Projects linked to the repository, and the owner's Projects that link back to it. A user-owned Project can be missing from the repository's own listing for a while after it is linked, so neither list alone is enough.

- **One Project:** use its `number` as `<n>`, its `owner` as `<owner>` and its `title` as `<project title>` in every command below. `repo` is the repository, as `owner/name`.
- **None:** say to run the `lets-setup-backlog` skill, and stop.
- **Two or more:** list each one's title and URL, ask the user to unlink the extras, and stop. Never pick one yourself.

Every issue joins the Project when you create it with `--project "<project title>"`. `gh issue create` prints the issue's URL, `<url>`; for an existing issue, get it with `gh issue view <i> --json url --jq .url`. Set the fields one at a time:

```bash
gh project item-edit <n> --owner <owner> --url <url> --field Kind --value Story
gh project item-edit <n> --owner <owner> --url <url> --field Priority --value Must
gh project item-edit <n> --owner <owner> --url <url> --field Size --value S
gh project item-edit <n> --owner <owner> --url <url> --field Status --value Todo
```

An issue is shaped when Kind, Priority and Size are set. Status Todo says nobody has taken it. The glossary documentation Chore below is the sole exception: assign it and set In Progress. Set Todo once, right after the other three, whenever you create or reshape a story, bug or chore; the Project's workflows and `lets-build` move it on from there. An epic gets a Kind and nothing else.

## The stance

- **Curious, not prescriptive.** Ask the questions the input raises.
- **Visual.** Draw journey flows and priority tables.
- **Grounded.** Explore the codebase, and annotate the plan with what exists today.
- **Adaptive.** Follow a thread, and change course when new information arrives.
- **Outcome-oriented.** Converge on concrete output: the map and the issues.
- **One phase at a time.** Confirm each phase before the next.
- **Checkpointed.** Write every confirmed phase at once, so the session can stop and resume at any phase boundary.

## The map

`discovery.md` describes the people the product serves and the paths they take through it. It is not a record of runs. Many runs edit the same map, so its size follows the number of personas and paths, never the number of runs. Every run keeps these rules:

- **Personas are durable.** A persona entry reads the same before and after any single epic ships. *Pain today* is the person's real-world problem without a good tool; *Success looks like* is the lasting outcome they want. Neither names code, milestones, or epics. Edit a persona only when facts about the person change.
- **Personas sit in audience groups.** Under `## Personas`, one `###` heading per audience — typically `People who use <product>` and `People who build <product>` — and one `####` entry per persona. Omit a group that has no personas. `## Journey Map` uses the same groups.
- **One journey per distinct path.** Personas who walk the same stages share one journey, named for all of them, with an *Implication* note saying what that means for the build. A stage only some of them reach stays in the shared journey and names whose it is, such as `Bump pins (Mo)`. A persona gets its own journey only when most of its stages differ.
- **A journey shows the full path.** Map every stage the persona goes through, not only the part this run's input covers. Draw future stages only from sources the project already has (vision, architecture docs, decision records); do not invent them.
- **Annotations show present status only.** A `supported` stage links no issues — the code is the evidence. A `partial` or `gap` stage links only the open issues that close it. A `gap` with no link is known but unplanned.
- **A freshness line dates the annotations.** The Journey Map opens with `Stage status checked against the code on YYYY-MM-DD.` Update it whenever you re-check stages.
- **Epic scope stays out.** A run's scope, exit criterion, and "what this release covers" go in its parent issue, never in the map. No section, heading, or persona is named for a milestone or epic.
- **Finished journeys stay.** A journey whose stages are all `supported` stays in the map; it still shows what the product does. Only a path that no longer applies is removed.
- **Removal is recorded.** Remove a persona or journey that no longer applies, and note the removal and its reason in the current run's parent issue.

## Locate the discovery

At the start:

1. Run `openspec list --json` and take the OpenSpec root from `root.path`. It must be this repository's root, or a directory inside it: `openspec list` answers with the nearest root, which can be a parent of the repository. If it is outside, stop and say the project has no OpenSpec root. Use the resolved root everywhere below, never a bare `./openspec/`.
2. Read `<root.path>/openspec/config.yaml` (or `config.yml`) if it exists. Its `context` and `rules` are constraints for you to follow, not content to reproduce. Read [Domain language](references/domain-language.md) and resolve the shared glossary from that root and context. Load ancestor and affected-scope glossaries as capabilities emerge, before using their terms.
3. Find the Project, as in [The Project](#the-project). The map's links need the owner and the name from its `repo`.
4. Pick the mode, first match wins:
   - **The user brings one bug, chore or small story, or names one issue to shape on its own**: [Shape one issue](#shape-one-issue). An issue that holds a large idea is the input to a discovery run instead. If you cannot tell which the user means, ask.
   - **An open parent issue says `Discovery status: in-progress`** (`gh issue list --state open --search '"Discovery status: in-progress" in:body' --json number,title,body`): [Resume mode](#resume-mode). If more than one does, list them and ask the user which to resume, or whether to start a new run. Never pick one yourself.
   - **`<root.path>/openspec/discovery.md` does not exist**: run every phase.
   - **Otherwise**: [Revision mode](#revision-mode).

If any of these fails, stop and show the error. Never write the map anywhere but the resolved root.

## Develop the vocabulary

Use the shared reference in discovery, revision, resume and single-issue
shaping. Before writing a packet that relies on a term, compare its meaning
with established vocabulary. For example, distinguish an authentication
Session from an existing game Session with the user. Keep unresolved terms
and candidate meanings explicitly tentative in the shaping issue.

Propose meaning and the narrowest scope covering actual uses, checking
ancestor definitions and siblings. Order used by checkout and fulfillment
belongs in their common scope; Checkout attempt used only by checkout stays
local. Confirm meaning and scope with the user, then write the accepted
entry immediately to its authoritative glossary. Subsequent packets use
that agreed concept. Qualify conflicting local meanings rather than silently
overriding shared ones; move newly shared definitions and update references
without duplicate authoritative copies. A stale declared glossary path must
be reported and resolved before dependent discussion continues.

## Preserve glossary edits between sessions

Before the first glossary write, record the starting branch and full commit,
working-tree and index status, and the original contents (or absence) of
each affected path in `.workspace/<run>-glossary-handoff.md`. Check each
additional path before writing it. Track exactly which edits this shaping
run owns. If a glossary already has user edits, resolve ownership with the
user before touching it. Separate or preserve those edits first; never
include them in an owned checkpoint merely because they share a path.
A checkpoint's base must contain the baseline needed to reproduce its patch.

At every confirmed phase checkpoint with glossary changes:

1. Inspect the owned diff against its baseline and the staged diff. Commit
   only owned glossary paths as a separate `docs:` unit. Use explicit paths
   (including new files); never `git add -A` or commit unrelated staged work.
   From the default branch, first create a holding branch named for the
   shaping issue from the recorded starting commit. Continue shaping there;
   subsequent checkpoints extend the same branch. Check branch existence
   before creation and never reset a branch to reuse its name.
2. If shaping began on an existing change branch, keep glossary checkpoints
   there as separate `docs:` commits. Do not create a holding branch from
   unrelated implementation history. Record each exact owned commit and its
   parent, so build imports only glossary commits with available context.
3. Publish the checkpoint branch to a chosen writable remote without a PR.
   Confirm the remote head contains the checkpoint. If no writable remote
   exists or publication fails, report the limitation, keep the local
   checkpoint and do not call it available to another checkout.
4. Update **Glossary handoff** in the shaping issue (or parent): starting
   branch/base, owned paths, ordered checkpoint commits and their parents,
   head, remote URL and remote branch, publication state, and owner still
   undecided. Keep tentative meanings separate. In discovery, record the
   checkpoint beside the status line too. Do this after each phase, not only
   at final handoff. The issue is the durable recovery record; scratch files
   alone are insufficient.

On resume, read the issue's handoff before deciding whether this run has
vocabulary edits. Fetch its recorded branch and verify the exact checkpoint
and path-only commit range, then restore that branch and its definitions
before continuing. Verify the baseline from its recorded base; do not rely
on conversation history or another checkout's `.workspace/`. Never force a
checkout or automatically stash unrelated changes. If local edits obstruct
recovery, preserve them and report the obstruction.

After the final checkpoint, restore owned paths to the recorded starting
baseline before normal build. Usually switch safely to the starting branch;
verify tracked owned paths have no diff and newly introduced paths are
absent. On an existing change branch, the separately committed vocabulary
stays in its PR; record that destination rather than undoing its commits.
For an explicit restoration, first compare each path and index with the
owned checkpoint. Restore only verified owned edits, deleting a newly
created file only when it still matches the checkpoint. If later user edits
exist, stop restoration of that path and resolve ownership. Never discard
those edits, reset the whole tree, or change unrelated paths. Retain the
checkpoint branch until the destination safely contains its definitions.

## The phases

**Checkpoint every phase.** Apply the glossary checkpoint procedure above whenever the phase changed definitions. When the user confirms a phase, write its result at once, and set the parent issue's status line to the next phase: `personas`, `journey-map`, `moscow`, then `stories`. A revision run that skips personas and the journey map goes from Ingest straight to `moscow`.

Personas and the journey map go in `discovery.md`. Everything else goes in the parent issue.

The parent issue and `discovery.md` are the resume point. A future session with none of this conversation must be able to pick up exactly where this one stopped.

### Phase 1: Ingest

Take in the input — a file path, an issue (`gh issue view <n>`), pasted text, or just conversation. Then:

- Restate the scope in your own words; have the user confirm
- Surface goals and non-goals
- List open questions as they emerge (they land in the parent issue)

**Checkpoint:** The parent issue is public, so confirm its text first. Write it from the parent template in [templates/issue-template.md](templates/issue-template.md): the status line, the input reference, the confirmed scope and any open questions so far. Title it `Epic: <capability>`, create it with `gh issue create --title "Epic: <capability>" --body-file <file> --project "<project title>"`, then set its Kind to `Epic` with `item-edit`. Phase 5 may turn it into a single story.

### Phase 2: Personas

Start from the personas already in `discovery.md`. For the people this input serves, reuse an existing persona first; extend its entry only if the input reveals a lasting fact about them. Add a new persona only for a person the map lacks; most runs add none. For each new persona:

- **Who** they are
- **Goal** they are trying to reach
- **Pain today** their real-world problem without a good tool
- **Success looks like** the lasting, observable signal they got value

Push back if personas are roles without needs ("admin") rather than people with goals, or if an entry would stop being true once this run ships (see [The map](#the-map)).

**Checkpoint:** write the Personas section of `discovery.md` (create the file from [templates/discovery-template.md](templates/discovery-template.md) if it is missing).

### Phase 3: Journey map

For each persona this run serves, find the journey they walk. Extend an existing journey — add stages, re-annotate, link new stories — before drawing a new one; draw a new journey only for a path the map lacks. Map the persona's full path, not only this run's slice. Draw it:

```
  Discover ──► Sign up ──► First run ──► Daily use ──► Share
     │            │            │             │           │
    gap          gap        partial      supported      gap
```

Annotate every stage against the **real codebase** — read the code, do not guess:

- `supported`: works today
- `partial`: exists but incomplete for this journey
- `gap`: does not exist

Cite the open issue that closes a gap as a full link, `[#N](https://github.com/<owner>/<repo>/issues/N)`; a bare `#N` does not link inside a repository file. A `supported` stage carries no link.

**Checkpoint:** write the Journey Map section of `discovery.md`, with today's date on its freshness line.

### Phase 4: MoSCoW

Bucket the capabilities the journey surfaced into **Must / Should / Could / Won't (this release)**.

- Every Must needs a reason tied to a persona's journey
- Push back on Must inflation: if everything is a Must, nothing is
- Won't is a decision, not a dumping ground — record why

**Checkpoint:** write the MoSCoW section of the parent issue.

### Phase 5: Stories

Cut the Must (then Should) work into stories:

- **Every story is a thin vertical slice:** end-to-end and demoable, never a horizontal layer. A persona can do something new when it ships.
- **Each story is one OpenSpec change.** Its future proposal's what and why fits in about 200 words. If the story packet already strains that budget, the story is too big: split it.
- **The first story is the walking skeleton:** the thinnest path through the whole journey.
- **Order by dependency first, then value.**
- **Size each story XS, S, M, L or XL.** L and XL are too big: split them.
- **Read the backlog before you cut:** `gh issue list --state all --limit 500 --json number,title,state,body,assignees`. Work that fits an open, unassigned story updates that issue instead of duplicating it.

Each story gets a **story packet**: the story template in [templates/issue-template.md](templates/issue-template.md). Every field is required, because the `lets-build` run that plans the story starts with none of this conversation.

**Size check-out:** after drafting the stories, review each against the ~200-word proposal budget. List every story that looks too large, each with 1-2 candidate split lines (or a one-line reason it genuinely resists splitting), and resolve with the user before finalizing.

**One story or many:**

- **The whole run fits one story.** The parent issue becomes that story. Retitle it `<kebab-story-name> — <one-line outcome>` and replace its body with the story template. Then set Kind `Story`, its Priority, its Size and Status `Todo`, with four `item-edit` commands. Keep the status line, input reference and complete **Glossary handoff** recovery record when replacing the body. Won't calls go in its Scope `out`.
- **More than one story.** The stories are sub-issues of the parent.

**Checkpoint:** Issues are public, so show the new and changed issues as a table (title, blocked by, MoSCoW, size) and confirm. Put each blocker on the story it blocks, never only on the parent, because `lets-build` cannot see a blocker on a parent issue when it picks the next story. When something blocks every story under the parent, give each story its own `--blocked-by` link. Then create them in dependency order, blockers first:

```bash
gh issue create --title "<title>" --body-file <file> --project "<project title>" --parent <parent#> [--blocked-by <n>,<n>]
```

For each issue:

1. The command prints the issue URL, `<url>`. The number is its last segment.
2. Set Kind `Story` with `item-edit`.
3. Set Priority to the story's MoSCoW value: `Must`, `Should` or `Could`.
4. Set Size to the story's size.
5. Set Status `Todo`.
6. Stop at the first error, including a failed field update.
7. Before you retry, search for the title with `gh issue list --state all --search "<title> in:title"`, so you never create a duplicate. If the issue exists but a field is unset, set it on that issue. Do not create the issue again.

### Phase 6: Finalize

Remove the status line from the parent issue. Keep the `Input:` line, so the run always names what it started from. Resolve every open question, or move it to the story it blocks. Link the journey stages in `discovery.md` to the stories that close them. `## Backlog` stays a single link to the repository's issues page, never a list of issues. Show the user the stories as a checklist of issue links.

Run [Glossary handoff](#glossary-handoff) before the ordinary handoff below.

Then hand off:

> The stories are GitHub issues under #<parent>. To start building, run `lets-build` on the default branch: it picks the next story, claims it and plans it. Run this skill again at any time to revise the plan.

## Shape one issue

Use this when the user brings one bug, chore or small story, or names an open issue that has no packet. It shapes that one issue. It never touches `discovery.md` and never starts a discovery run. Another skill can send work here too, such as `lets-brainstorm` when it splits a story; it gives you the issue's parent and what blocks it.

1. **Find or create the issue.** When the user named one, read it with `gh issue view <i> --json title,body,url,parent`. Otherwise search for one first: `gh issue list --state open --search "<words> in:title" --json number,title`. If one matches, ask whether it is the same work. If none does, you create it in step 6. If the issue has a **Glossary handoff**, recover its recorded checkpoint before continuing or deciding whether this run has glossary edits. An existing issue that is not on the Project yet joins it with `gh project item-add <n> --owner <owner> --url <url>`.
2. **Pick the Kind, and confirm it.** A Bug is something that should work and does not. A Chore is work no persona sees, such as a dependency bump, a CI fix or a tidy-up. A Story is something a persona can do once it ships.
3. **Read the code the issue names**, and search for more. Load shared, ancestor and affected-scope vocabulary and develop definitions as above. **Relevant code** lists what you find, or `new` when the story builds on nothing that exists.
4. **Interview for the Kind's template** in [templates/issue-template.md](templates/issue-template.md), one question at a time: the story template for a Story, the bug template for a Bug. A Chore has no template. It needs only its one-line outcome, and a title in the template's shape. When `discovery.md` exists, use its persona and journey names, but do not edit it.
5. **Size it** XS, S, M, L or XL. L or XL is too big for one change: say so, offer a discovery run on it instead, and stop.
6. **Confirm, then write.** The issue is public, so show the user its title, body, Kind, Priority and Size, and get a yes. Write the packet, retaining any existing **Glossary handoff** recovery record when replacing the body. For an existing issue, use `gh issue edit <i> --title "<title>" --body-file <file>`. For a new one, use `gh issue create --title "<title>" --body-file <file> --project "<project title>" [--parent <p>] [--blocked-by <i>,<i>]`. Then set Kind, Priority, Size and Status `Todo`, in that order, with `item-edit`. A Chore and a Bug get a Priority and a Size too.

Checkpoint any glossary edits and run [Glossary handoff](#glossary-handoff). Then say that `lets-build` can take it: on the default branch, it picks the next issue, or the user can name this one.

## Resume mode

When an open parent issue says `Discovery status: in-progress` (or the user picked one of several):

1. **Read it and `discovery.md`.** Recover any recorded glossary checkpoint as above first. The status line records the next phase; the sections already present are confirmed work.
2. **Restate where discovery stopped** — a one-paragraph summary of what the parent issue and map already capture — and confirm with the user.
3. **Continue from the recorded phase.** Do not re-litigate confirmed sections unless the user brings new information; then revisit and update them.

## Revision mode

When `discovery.md` exists and no run is in progress:

1. **Read it**, then reconcile the map against reality:
   - `gh issue list --state all --limit 500 --json number,title,state,stateReason,parent` lists shipped and open stories
   - Find the map's links to closed issues. This prints the number of each closed issue the map still links:

     ```bash
     grep -oE 'issues/[0-9]+' <root.path>/openspec/discovery.md | cut -d/ -f2 | sort -u \
       | grep -xFf <(gh issue list --state closed --limit 500 --json number --jq '.[].number')
     ```

     No output means no link is stale. Re-check each one's stage against the code, then do one of:
     - the stage now works: upgrade it and drop the link;
     - an open issue now closes the gap: link that one instead;
     - nothing open does: drop the link and leave the stage an unplanned `gap`.
   - Re-annotate every journey stage against the code; stages move to `supported` as their stories ship
   - Update the freshness line to today
2. **Take in what the user brings** — a new PRD, new requirements, changed priorities, learnings from shipped stories. New work is a new run with its own parent issue: Ingest, then MoSCoW and Stories. Revisit Personas and the Journey Map only when the input changes who the product serves or how they use it.
3. **Priorities of existing stories are revisable too:** update the story's MoSCoW line, with its reason, and set its Priority with `item-edit` in the same step.
4. **Never silently delete a story.** Close a superseded story as not planned with a one-line reason, so the plan's history stays legible: `gh issue close <n> --reason "not planned" --comment "Superseded by #<m>: <reason>"`.

## Glossary handoff

Work through these steps in order. "You" is the agent; "the user" is the
person shaping with you.

### Decide whether to ask

1. Compare this run's owned glossary paths and checkpoints with the
   starting baseline. Include checkpoints you recovered on resume. Ignore
   unrelated changes.
2. If this run made no glossary edits, use the ordinary handoff. Ask no
   glossary question, and skip the rest of this section.

### Ask who carries the edits

3. Ask the user which pull request carries the edits. Do not ask again
   whether the meanings are accepted.
   - For one story, offer its build branch and pull request, or a
     documentation pull request if the user stops after shaping. Create
     that pull request only if the user chooses it.
   - For definitions several stories share, offer one story's pull
     request, or a documentation pull request that lands first.
4. Pick exactly one owner. Never copy the edits into other pull requests,
   and never stack independent work automatically.
5. If you are shaping on a change branch and split off another story, the
   definitions stay in the original change's glossary commit, unless the
   user picks a safe alternative. When the split story needs them, record
   the original issue and pull request as the owner. Step 3 still applies.

### Record the choice

6. In the owner issue, or the shaping parent, extend **Glossary handoff**
   with: the owner issue and pull request, the affected paths, the base,
   the remote URL and branch, the full ordered checkpoint range, and
   whether it is published.
7. In each affected packet, point **Depends on** at that record. Never
   copy definitions into issue bodies.
8. If the user stops before choosing an owner, record that the choice is
   pending. The work stays an unresolved prerequisite.

### Set up a documentation pull request

Only when the user chose a documentation pull request:

9. Confirm the issue and the pull request content with the user.
10. Create a Chore issue with a one-line documentation outcome. Set Kind
    Chore, a Priority and a Size with the Project operations above, run
    `gh issue edit <i> --add-assignee @me`, and set Status In Progress.
    Never leave it Todo for another build to pick.
11. If the assignment or a field update fails, tell the user the setup is
    incomplete, and do not call it ready. Resume on the same issue. Never
    create a second one.
12. Use the verified glossary-only holding branch, or move its owned
    commits to a clean documentation branch. Open the pull request with
    `Closes #<chore>`. It never closes the implementation story or an
    unfinished parent.
13. This pull request creates no OpenSpec change. It still uses separate
    `docs:` commits, runs the project's checks before you push, follows
    the finishing rules that apply, and leaves the merge to the user.
    Never commit to the default branch. A change branch that holds code
    cannot become this pull request; take only the verified glossary
    commits.

### Link the blocked stories

14. Ask the user before you add blockers. Then, on each other story that
    needs the unmerged definitions, run
    `gh issue edit <dependent> --add-blocked-by <owner>`. Never put the
    blocker only on the parent, and never use a "relates to" link.
15. If the user declines the links, make the dependent story's
    **Depends on** name the owner, the definitions it needs and the
    unresolved prerequisite. Do not call the story ready. Build's
    glossary check must still stop it.
16. Independent stories can use the definitions once the owner's pull
    request merges.

### Hand over

17. Restore the owned paths to the baseline, as
    [Preserve glossary edits between sessions](#preserve-glossary-edits-between-sessions)
    says, before normal build. Keep the holding branch until the
    destination holds its edits. After the transfer, record the
    destination branch, commits and pull request in the issue's
    **Glossary handoff**.
18. If a checkpoint is unpublished or missing, resolve that first. It
    never permits writing competing definitions in another checkout.

## Handing off to lets-build

- Discovery **never** creates changes. `lets-build` creates them, one story at a time.
- `lets-build` picks the next story, and owns the selection rule. The user can also name an issue for it.
- The story packet is the input — it must stand alone.
- One story per change. If the user asks you to batch-create proposals for all stories, decline and explain: proposals written ahead of implementation go stale and overlap on shared specs.
