## 1. Route Dependabot maintenance PRs

- [ ] 1.1 [easy] Accept PR events targeting either maintained branch, excluding other targets (verify: parse the workflow with Python/PyYAML and check its event filters).
- [ ] 1.2 [easy] Check PR freshness against its base branch (verify: execute the workflow shell step with a recording `gh` stub for both targets, behind and current heads, and an update failure).
