# The Sidereal Spec-Driven Development Workflow

The best way to contribute complex features to Sidereal using Claude Code is to follow this structured five step process designed to help guide you through the development cycle.

## Steps in the Development Cycle

These 4 steps are used iteratively for each new feature (or contribute). You can use all of them or pick and choose based on your needs.

### 1. Plan Spec
The [`/plan-spec`](.claude/commands/plan-spec.md) command helps you take a rough idea and shape it into well-scoped requirements. Use this when you need clarification before officially writing up a feature.

This command initializes a new spec folder with proper structure, researches your requirements through targeted questions, collects visual assets, identifies existing code patterns to reuse, and documents everything for reference in the next step ([write-spec](#2-write-spec)).

The `plan-spec` command creates a dated spec folder in the specs folder: [docs/planning/specs/](docs/planning/specs/)

```
docs/planning/specs/YYYY-MM-DD-my-new-spec/
└── planning/
    ├── visuals/
    └── requirements.md
```

The `plan-spec` command will then conduct an interactive research dialogue that:
- Analyzes context ([vision](docs/planning/vision.md), [roadmap](docs/planning/roadmap.md), [architecture](docs/planning/architecture.md))
- Asks you targeted clarifying questions
- Requests visual assets (mockups, wireframes, screenshots) for planning/visuals
- Identifies similar existing features to reference
- Automatically scans for and analyzes visuals
- Asks follow-up questions if needed

The agent will then document all of this information and create `docs/planning/specs/[this-spec]/planning/requirements.md` with transcription of your Q&A, visual analysis, functional/non-functional requirements, scope boundaries

It will begin by looking at the [roadmap](docs/planning/roadmap.md) to see what the next feature might be. You can confirm that's the one, or provide your raw description of your next new feature instead. (It's perfectly fine to deviate from the roadmap!)

### 2. Write Spec
The [`/write-spec`](.claude/commands/write-spec.md) command transforms your requirements into a detailed specification document.

The `write-spec` command creates one core document in `docs/planning/specs/[this-spec]/`:

- `spec.md` - Comprehensive specification with user stories, requirements, reusable components, technical approach, visual references, and scope boundaries

### 3. Create Tasks
The [`/create-tasks`](.claude/commands/create-tasks.md) command breaks down your spec into an actionable task list, grouped, prioritized, and ready for implementation.

The `create-tasks` command creates one core document in `docs/planning/specs/[this-spec]/`:

- `tasks.md` - Task breakdown with tasks grouped by specialty (database, API, UI, testing), strategically ordered, typically following a test-driven development approach (TDD)

### 4. Implement Tasks
The [`/implement-tasks`](.claude/commands/implement-tasks.md) command provides simple, straightforward implementation with your main agent. Perfect for smaller features or when you want direct control.

The `implement-tasks` command involves implementing the tasks defined in your tasks.md file. The agent will work through the task groups and check them off as they're completed.


## Other Commands

### Reconciliation
The [`/reconcile-spec`](.claude/commands/reconcile-spec.md) command verifies that the current state of implementation aligns with specifications and requirements.

You can run this command at any time to confirm a specification has been implemented correctly.


### Code Review
The [`/review-code`](.claude/commands/review-code.md) command performs a code review to verify quality, standards adherence, and spec.

It will review one of the follow scopes, and provide feedback:
- Recent changes (since last commit)
- Current branch/PR (all changes vs main)
- Specific files (provide paths)
- Feature area (related to a spec)
