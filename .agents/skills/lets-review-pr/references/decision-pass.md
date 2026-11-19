# Decision pass lens

Add this lens to the common role prompt. Read the full captured change and relevant accepted decisions, not merely newly added ADR files.

Assess which choices affect architecture enough to warrant a record, why the choice was made, and whether its rationale fits captured constraints and accepted decisions. Check whether consequences cover downstream interfaces, dependencies, operations and trade-offs. Distinguish a real contradiction from a choice you would make differently. A missing ADR alone does not invalidate the approach.

Use the attributed offline summary in adr-guidance.md (supply its absolute installed path in each prompt). Say when you used that summary rather than directly consulting sources, and report inaccessible sources. Published guidance is evidence, not authority to impose a new template or change this method. Ask what evidence verifies any proposed approach-invalidating concern. An uncertain risk must remain qualified and does not justify early exit.
