# The step prompt

Read this file when the run writes the prompt for a step.

## Write the prompt

If the folder is missing, create it first:
`mkdir -p .workspace/lets-go`. Then copy the frame below to
`.workspace/lets-go/<session>-prompt.md`. Fill in each `<...>`.
`<schema>`, `<mode>` and `<route line>` are the ones
[Enter the branch](../SKILL.md#enter-the-branch),
[Set the question mode](../SKILL.md#set-the-question-mode) and
[Route](../SKILL.md#route) set. For shape, `<mode>` is the one
[Run the shape step](../SKILL.md#run-the-shape-step) set. Under
`## What to run`, put this sentence. `<skill>` is the skill in the
step's row of [The step table](../SKILL.md#the-step-table):

```text
Invoke the `<skill>` skill for the change `<name>`, for issue #<i>, and stop where it stops.
```

For shape, put the text of [What shape runs](shape.md#what-shape-runs)
under `## What to run` instead.

For shape, the first two lines differ. There is no change, the branch is
the default branch, `<default>`, and [Route](../SKILL.md#route) has not
run, so there is no schema and no route line. For an issue, they are:

```text
You run one step of a lets-go run: shape, for issue #<i>.
Issue: #<i>, <issue-url>. Branch: <default>.
```

For an idea, they are:

```text
You run one step of a lets-go run: shape, for an idea.
Issue: none yet. The idea is: <idea>. Branch: <default>.
```

Resolve the installed
[Plain language](../../lets-shape/references/plain-language.md)
reference. Use its absolute path as `<plain-language-path>` in the
frame. The step has none of this session's conversation.

The rest of the frame is the same for every step.

````text
You run one step of a lets-go run: <step>, for the change <name>.
Issue: #<i>, <issue-url>. Branch: <name>. Schema: <schema>. <route line>

## What to run

<the sentence above>

## Writing

Plain-language method: <plain-language-path>.
Write for the people and agents who need this result. Give them enough
context to find the relevant information, understand it and take the
next action. Preserve exact report keys, markers and allowed values.
Make evidence, reasons and questions clear without changing permissions.

## Questions

The question mode is <mode>. At auto, the lets-go session answers each
question that has a recommendation, tells the user what it chose, and
asks the user the rest. At ask, it puts every question to the user, with
your recommendation as the suggested answer. Your rules are the same in
both modes. Never decide a question yourself. Never ask the user in this
session: nobody here can answer.

When you need a decision, stop. End with a needs-input report that lists
every open question, in the order you would ask them. Give a
recommendation where you have one. Give none for a question that asks
the user to confirm something, such as a brainstorm ledger, an issue
packet or a discovery phase.

A resumed step ends with an `## Answers` section. Each answer settles its
question, so never ask it again. An answer that ends with "(lets-go
accepted the step's recommendation; the user did not decide)" is not the
user's: record it as Assumption in a brainstorm ledger. Record any other
answer as User. An answer of yes to the confirmation question confirms
the ledger: the brainstorm may set `Confirmed by user: yes`.

## The report

End your final message with a step report: a fenced block tagged
step-report, holding one YAML mapping. Use one of these three:

```step-report
outcome: done
evidence:
  - "commit 3f2a1c0: feat: add the thing"
  - "pull request #12"
```

```step-report
outcome: needs-input
questions:
  - question: Which schema should the change use?
    recommendation: lets-minimal
  - question: Do you confirm the ledger?
```

```step-report
outcome: failed
reason: The tests fail on main before any change.
```

A done report's evidence lists the commits you made, and the pull request
when you opened or changed it. For shape, its first line is the issue,
as `issue #<i>: <title>`, or the parent, as `epic #<parent>: <title>`,
with a `story #<story>: <title>` line for each story. For review plan,
its first line is the verdict and the commit reviewed, as
`Plan review: <verdict> for <sha>`. Write the report last. Quote a line that
holds `#` or `: `. Put no other step-report block in the message.
````
