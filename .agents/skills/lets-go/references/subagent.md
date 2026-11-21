# The subagent step

Read this file when `lets delegate step run` exits 3, and the meta file
names a saved prompt.

## Run a step in a subagent

Take this step only when `lets delegate step run` exits 3, and the meta
file names the prompt `lets` saved,
`.workspace/lets-delegate/<session>.prompt.md`. That file holds the
prompt, with any answers added.

1. Run that prompt in a subagent that starts with no context from this
   session. It follows the saved prompt, works only in this repository
   on the branch `<name>`, or on the default branch for shape, and makes
   only the changes that the step calls for.
2. Read the last `step-report` block in the subagent's final message, by
   the rules `lets` uses:
   - The block opens with a line of three or more backticks followed by
     `step-report`. It closes with a line of at least as many backticks,
     and nothing else.
   - It holds one YAML mapping, whose `outcome` is `done`, `needs-input`
     or `failed`.
   - A `done` report holds `evidence`, a non-empty list of strings.
   - A `needs-input` report holds `questions`, a non-empty list of
     mappings. Each has a string `question`, and may have a string
     `recommendation`.
   - A `failed` report holds a string `reason`.
3. A final message that breaks any of these rules is a `failed` report.
   Its `reason` says what is missing or wrong, such as no `step-report`
   block, or a `done` report with no `evidence`.
4. Add `date` to a `done` report: the current time in ISO 8601 UTC, from
   `date -u +%Y-%m-%dT%H:%M:%SZ`.
5. Write the report as one JSON line, `outcome` first, to
   `.workspace/lets-go/<session>-report.json`.
