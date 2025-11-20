# Code Review Command

You are performing a code review to verify quality, standards adherence, and spec conformance.

## Workflow

### Step 1: Determine Review Scope

Ask user what to review if not specified:

```
What would you like me to review?

Options:
1. Recent changes (since last commit)
2. Current branch/PR (all changes vs main)
3. Specific files (provide paths)
4. Feature area (related to a spec)

If reviewing against a spec, which spec? [Or specify "general review"]
```

WAIT for user response.

### Step 2: Offer Reconciliation (if spec-based review)

If reviewing against a spec, offer to reconcile first:

```
Would you like me to run reconciliation before code review?

Reconciliation verifies all spec requirements are implemented.
Code review checks code quality and standards.

Options:
1. Reconcile first, then review
2. Just review code
```

If user chooses reconciliation, invoke `/reconcile` first, then proceed to code review.

### Step 3: Invoke Code Reviewer Agent

Once scope is determined, use the **code-reviewer** subagent:

Provide the code-reviewer with:
- Review scope (recent changes / specific files / branch / feature area)
- Spec path (if reviewing against a spec)
- Reconciliation results (if reconciliation was run)
- Any specific concerns user mentioned

The code-reviewer will:
- Analyze the code
- Verify spec conformance (if applicable)
- Consider reconciliation findings (if available)
- Generate detailed review report

### Step 4: Present Results

After code-reviewer completes, present the summary to user:

```
Code Review Complete!

**Assessment**: [✅ Approved | ⚠️ Needs Work | ❌ Major Issues]

**Issues Found**:
- 🔴 Critical: [N]
- 🟡 Important: [N]
- 🟢 Nice to Have: [N]

**Highlights**:
[Brief mention of strengths]

[If critical issues] ⚠️ Address critical issues before merging
[If approved] ✅ Code meets standards and spec requirements

Full report: [location if saved to file]
```

### Step 5: Offer Next Steps

Based on review results:

**If issues found**:
```
Would you like me to:
1. Help fix the critical issues
2. Explain any findings in detail
3. Re-review after you make changes
```

**If approved**:
```
✅ Code review passed!

[If implementing a spec]
NEXT STEP 👉 Update implementation checklist to mark this complete

[If pre-merge]
NEXT STEP 👉 Ready to merge
```

## Review Modes

### Mode 1: Recent Changes
Reviews commits since last push or specific commit range.

### Mode 2: Branch/PR Review
Compares current branch against main/base branch.

### Mode 3: Specific Files
Reviews only the files user specifies.

### Mode 4: Feature/Spec Review
Reviews all code related to a particular spec or feature area.

## Important Notes

- Always invoke the **code-reviewer** agent to perform the actual review
- Present results clearly with actionable next steps
- Categorize issues by severity
- Balance criticism with recognition of good work
- Reference specific standards that were checked
