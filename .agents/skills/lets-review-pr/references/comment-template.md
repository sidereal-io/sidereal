# PR review comment

Fill this template in plain language. Start with the AI disclosure and the useful
result. Explain the concern before the review process. Omit unused placeholders.
Put supporting review records in the details section. Keep each material
limit beside the concern it qualifies so readers can judge the finding
without opening the details. Do not publish a transcript or an approval
verdict.

```markdown
AI-generated review using <actual model names>, posted through this GitHub account.

Reviewed <PR URL> at [<short SHA>](<full commit link>).
<Head check: unchanged when checked at UTC time / newer commits were not reviewed /
current head could not be checked.>

<For each concern, ordered by potential impact:>

1. **<plain-language concern>** — Potential impact: <level and consequence>.
   Confidence: <level and what supports it>.

   <What the code or docs do, with links at the reviewed commit.>
   <Why it matters, including a concrete scenario. Separate what was verified
   from what could happen. Say what remains unknown.>
   Suggested next step: <a practical fix or question for the author>.

<For a clean result: No substantive supported concern survived the completed passes.
Say plainly that no significant concern was found within the stated scope;
this is not an approval.>
<For early exit: explain the verified problem with the approach and why code
review stopped.>
<Questions that need a person's decision, including competing evidence.>

<details>
<summary>Review details and limits</summary>

Reviewed head: <full SHA and commit link>.
Captured base: <full SHA and ref>; merge base: <full SHA>.
Capture interval: <UTC start/end>. Sources: <PR/issue/comment IDs and URLs,
edit/collection times or compact source list identifying the captured inputs>.
These sources were collected separately, so they may reflect different moments.

Completed checks: <decisions, implementation, focused questions; name anything skipped>.
Stopping reason: <no further supported question / verified problem with the approach /
review limit reached / missing evidence>.
Models: <actual models by role; known implementer models or unknown;
explain any limits on choosing different models and any offline summaries used>.
Limits: <what was read or run, what was unavailable, and what remains uncertain>.
Interpretation: <only if needed to distinguish a conclusion from the verified facts>.
<Omitted detail, if any; preserve important uncertainty and unresolved questions.>

</details>
```
