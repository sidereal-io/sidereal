# Reconcile Command

You are performing a reconciliation to verify that implementation aligns with specifications and requirements.

## Purpose

Reconciliation ensures:
- All spec requirements are implemented
- Implementation reports accurately reflect code
- No scope creep or missing features
- Deviations are documented and acceptable

## Workflow

### Step 1: Identify the Spec

Determine which spec to reconcile:

```bash
# If user provided spec name/path, use that
# Otherwise, list recent specs
ls -t docs/planning/specs/*/spec.md | head -5
```

If ambiguous, ask:
```
Which spec would you like to reconcile?

Recent specs:
1. [spec-name-1] - [date]
2. [spec-name-2] - [date]
3. [spec-name-3] - [date]

Or provide the spec path/name.
```

WAIT for user response if needed.

### Step 2: Verify Spec Has Implementation

Check if implementation has begun:

```bash
SPEC_PATH="docs/planning/specs/[spec-name]"

# Check for implementation artifacts
ls $SPEC_PATH/implementation/ 2>/dev/null
```

If no implementation folder exists:
```
⚠️ No implementation found for this spec.

The spec exists but hasn't been implemented yet.

Would you like to:
1. Start implementation with /implement-spec
2. Reconcile anyway (will show everything as unimplemented)
```

### Step 3: Invoke Reconciliation Agent

Use the **reconciliation** subagent:

Provide the reconciliation agent with:
- Spec path
- Current git branch and commit (for context)

The reconciliation agent will:
1. Load all documentation (requirements.md, spec.md, progress.md, checklist.md)
2. Search codebase for implementation evidence
3. Build traceability matrix
4. Calculate completeness score
5. Identify gaps and deviations
6. Generate comprehensive reconciliation report

### Step 4: Present Results

After reconciliation agent completes, present summary:

```
Reconciliation Complete!

**Spec**: [spec-name]
**Overall Completeness**: [XX]% [✅ | ⚠️ | ❌]

**Requirements Status**:
✅ Fully Implemented: [N]
🟡 Partially Implemented: [N]
⚠️ Deviations: [N]
❌ Not Implemented: [N]

**Gaps**:
🔴 Critical: [N]
🟡 Important: [N]
🟢 Minor: [N]

**Recommendation**: [✅ Approve | ⚠️ Approve with Follow-up | ❌ Needs Work]

[If critical gaps or deviations]
⚠️ Review the reconciliation report for details on gaps and deviations

Full report saved to:
docs/planning/specs/[spec-name]/implementation/reconciliation.md
```

### Step 5: Offer Next Steps

Based on reconciliation results:

**If 100% complete and approved**:
```
🎉 Reconciliation shows 100% conformance!

All spec requirements are implemented and verified.

Ready to merge? Consider running /review-code for final quality check.
```

**If mostly complete (80-99%)**:
```
✅ Implementation is largely complete

Remaining work:
- [Gap 1]
- [Gap 2]

Would you like to:
1. Continue implementation to address gaps
2. Review deviations to assess if acceptable
3. Update spec to reflect actual implementation
```

**If significant gaps (<80%)**:
```
⚠️ Significant gaps found

Implementation is [XX]% complete with [N] requirements missing.

Recommended action:
1. Review reconciliation report: docs/planning/specs/[spec-name]/implementation/reconciliation.md
2. Continue implementation with /implement-spec
3. Address critical gaps first
```

**If deviations found**:
```
⚠️ [N] deviations from spec detected

Some implementations differ from the specification.

Review deviations in the report:
- [N] acceptable deviations (no action needed)
- [N] require review (discuss with team)
- [N] should be fixed (bring into conformance)
```

### Step 6: Documentation Sync (if needed)

If discrepancies found between implementation reports and code:

```
📝 Documentation discrepancies detected

Implementation reports don't match actual code:
- [N] over-reported (documented but not implemented)
- [N] under-reported (implemented but not documented)

Would you like me to:
1. Update implementation reports to match code
2. Review each discrepancy individually
```

## Use Cases

### Use Case 1: Pre-Merge Verification
```
User: "/reconcile before I merge"
→ Verifies all requirements implemented
→ Checks for scope creep
→ Ensures docs are accurate
→ Provides go/no-go recommendation
```

### Use Case 2: Post-Implementation Check
```
User: "I finished implementing the spec, let's reconcile"
→ Calculates completeness score
→ Identifies any missed requirements
→ Validates against original requirements
```

### Use Case 3: Mid-Implementation Status
```
User: "/reconcile to see where we are"
→ Shows partial completion status
→ Lists remaining work
→ Tracks progress over time
```

### Use Case 4: Maintenance Audit
```
User: "/reconcile the original auth spec"
→ Verifies old features still conform
→ Checks if maintenance broke conformance
→ Validates docs are still accurate
```

## Important Notes

- **Thorough but not perfect**: Reconciliation searches for evidence but may miss creative implementations
- **Requires context**: Best used when spec and implementation are both complete
- **Actionable output**: Always provides clear next steps
- **Living document**: Reconciliation report can be regenerated as code evolves
- **Deviation isn't failure**: Acceptable deviations are normal in development

## Integration with Other Commands

**With /implement-spec**:
```
/implement-spec → [implementation work] → /reconcile → verify completeness
```

**With /review-code**:
```
/reconcile → identify gaps → fix gaps → /review-code → merge
```

**Standalone**:
```
/reconcile → get status report → continue work based on findings
```
