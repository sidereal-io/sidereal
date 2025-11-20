# Spec Implementation Workflow

You are guiding the implementation of a specification. This workflow helps developers build features systematically while tracking progress and maintaining quality.

## Workflow Overview

This process follows 4 main phases:

1. **Setup** - Load spec and create implementation structure
2. **Build** - Implement requirements iteratively
3. **Track** - Document progress and decisions
4. **Verify** - Ensure all requirements are met

## Multi-Phase Process

### PHASE 1: Setup and Planning

#### Step 1.1: Identify the Spec

Determine which spec to implement:

```bash
# If user provided spec name/path, use that
# Otherwise, list available specs
ls -t docs/planning/specs/*/spec.md | head -5
```

Ask user which spec to implement if ambiguous.

#### Step 1.2: Load Spec and Context

Read the specification:
```bash
# Read the spec
cat docs/planning/specs/[spec-name]/spec.md

# Check for requirements context
cat docs/planning/specs/[spec-name]/planning/requirements.md

# Check for visual assets
ls docs/planning/specs/[spec-name]/planning/visuals/ 2>/dev/null
```

#### Step 1.3: Create Implementation Structure

If not exists, create:
```bash
mkdir -p docs/planning/specs/[spec-name]/implementation
```

Create initial checklist at `docs/planning/specs/[spec-name]/implementation/checklist.md`:

```markdown
# Implementation Checklist: [Spec Name]

Created: [Date]

## Functional Requirements

[For each requirement in spec.md]
- [ ] [Requirement name]

## Technical Tasks

- [ ] Database schema/migrations (if needed)
- [ ] API endpoints (if needed)
- [ ] Backend services/logic
- [ ] UI components (if needed)
- [ ] Validation implementation
- [ ] Error handling
- [ ] Tests written
- [ ] Documentation updated

## Quality Checks

- [ ] Follows coding style standards
- [ ] Validation per validation.md
- [ ] Error handling complete
- [ ] Code review performed
- [ ] Spec conformance verified

## Out of Scope

[List items from spec's "Out of Scope" section as reminder]
```

#### Step 1.4: Present Implementation Plan

Output to user:

```
Ready to implement: [Spec Name]

📋 Requirements loaded: [N requirements]
🎨 Visual assets: [Found X files / None]
📂 Implementation tracking: Ready

## Requirements Overview:
1. [Requirement 1 name]
2. [Requirement 2 name]
[...]

## Suggested Implementation Order:
[Based on dependencies, suggest order like:]
1. Start with [foundation/database/API] because [reason]
2. Then build [core logic] because [reason]
3. Finally add [UI/integration] because [reason]

Ready to start? I'll guide you through implementation and track progress automatically.
```

WAIT for user confirmation before proceeding.

### PHASE 2: Iterative Implementation

You will now guide the user through building the feature. This is a conversational, iterative process.

#### Step 2.1: Work on Requirements

For each requirement or task:

1. **Clarify**: Ensure you understand what needs to be built
2. **Check for Reusable Code**: Reference "Existing Code to Leverage" from spec
3. **Build**: Write the code
4. **Validate**: Ensure validation.md principles are followed
5. **Handle Errors**: Apply proper error handling
6. **Test**: Write minimal tests per testing.md (if at logical completion point)

#### Step 2.2: Update Checklist as You Go

After completing each requirement or significant task, update the checklist:

```bash
# Update docs/planning/specs/[spec-name]/implementation/checklist.md
# Mark completed items with [x]
```

Inform user of progress:
```
✅ Completed: [Requirement name]

Progress: [X/Y] requirements complete
Next: [Next requirement name]
```

#### Step 2.3: Track Technical Decisions

When making architectural or design decisions:

Output to user:
```
📝 Technical Decision: [Decision name]

Decision: [What you decided]
Rationale: [Why this approach]
Alternatives: [Other options considered]

This will be documented in the implementation report.
```

Continue until user indicates a natural checkpoint or completion.

### PHASE 3: Track Implementation Progress

At natural checkpoints (user request, or after completing several requirements), invoke the **implementation-tracker** agent:

Provide the tracker with:
- Spec path
- Recent work completed
- Any deviations from spec

The tracker will update `docs/planning/specs/[spec-name]/implementation/progress.md`.

Inform user:
```
📊 Progress tracked!

Status update saved to implementation/progress.md
```

### PHASE 4: Verification and Completion

#### Step 4.1: Verify All Requirements Met

When implementation is complete (or user requests verification):

1. Read the checklist: `docs/planning/specs/[spec-name]/implementation/checklist.md`
2. Compare against spec.md requirements
3. Identify any incomplete items

Output to user:
```
🔍 Verification Report

Requirements: [X/Y] complete
Tests: [Status]
Documentation: [Status]

✅ Complete:
- [Requirement 1]
- [Requirement 2]

⚠️ Incomplete:
- [Missing requirement if any]

🎯 Out of Scope (correctly excluded):
- [Items from out of scope list]
```

#### Step 4.2: Final Implementation Tracking

Invoke **implementation-tracker** one final time for complete documentation.

#### Step 4.3: Offer Reconciliation

Offer to reconcile implementation against spec:
```
Implementation complete!

✅ All [Y] requirements implemented
✅ Implementation tracked and documented

Would you like me to:
1. Run `/reconcile` to verify all requirements are met
2. Run `/review-code` to check code quality and standards
3. Both (reconcile first, then review)
```

If user chooses reconciliation, invoke the `/reconcile` command.

#### Step 4.4: Recommend Code Review

After reconciliation (if run) or if user skips, suggest code review:
```
RECOMMENDED NEXT STEP 👉 Run `/review-code` to verify quality and standards adherence.
```

## Important Guidelines

### During Implementation

**Reuse Existing Code**:
- Check spec's "Existing Code to Leverage" section
- Search for similar patterns before writing new code
- Follow established patterns in the codebase

**Stay in Scope**:
- Refer to spec's "Out of Scope" section
- Don't implement excluded features
- Focus on defined requirements only

**Communicate Decisions**:
- Explain technical choices
- Note deviations from spec with rationale
- Ask for clarification when spec is ambiguous

### Tracking and Documentation

**Update Checklist Regularly**:
- Mark items complete as you finish them
- Keep user informed of progress
- Use checklist to prevent missing requirements

**Invoke Tracker at Checkpoints**:
- After completing major sections
- When making significant technical decisions
- Before finishing implementation
- When user requests progress update

**Document Deviations**:
- If you deviate from spec, explain why
- Document in implementation notes
- Suggest spec update if needed

### Quality Practices

**Write Minimal Tests During Development**:
- Per testing.md, don't test every change
- Focus on completing implementation first
- Add strategic tests at logical completion points
- Critical business logic should have tests

**Validate Input Properly**:
- Server-side validation is mandatory
- Follow validation.md principles
- Validate early, fail fast
- Provide clear error messages

**Handle Errors Gracefully**:
- Catch and handle errors appropriately
- Log errors with proper context
- Return meaningful error responses
- Don't let errors crash the application

## Example Implementation Flow

### Example 1: Full Implementation Cycle

```
User: "/implement-spec for the image-upload feature"