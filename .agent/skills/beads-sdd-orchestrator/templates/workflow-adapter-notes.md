# Workflow Adapter Notes

Use this file to capture project-specific mappings from an SDD workflow into Beads.

## Project workflow

Selected workflow:

- [ ] Spec Kit
- [ ] Superpowers
- [ ] BMad
- [ ] GSD
- [ ] Custom

## Artifact mapping

| Artifact | Path / Command | Beads mapping |
|---|---|---|
| Spec/design |  | Parent feature bead spec |
| Implementation plan |  | Temporary worksheet + plan summary |
| Task list |  | Child task beads |
| Execution workflow |  | One child bead per execution slice |
| Review/QA |  | Gates + acceptance verification |

## Commands

### Sync/prime

```bash
bd dolt pull
bd prime
```

### GitHub mirror

```bash
bd github sync --push-only --prefer-local
```

Replace with project-specific commands if different.

## Notes

