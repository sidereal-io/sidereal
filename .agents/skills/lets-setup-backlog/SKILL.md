---
name: lets-setup-backlog
description: Create or check the GitHub Project this repository uses as its backlog. Use when a repository has no Project, when lets-shape or lets-build say to run this skill, or to check that an existing Project has the fields the backlog skills need.
---

# Let's set up the backlog

The backlog is the one open GitHub Project linked to this repository. `lets-shape` and `lets-build` read and write its fields by name. This skill creates that Project, or checks that an existing one has what they need. Run every command below from the repo root.

`gh project` commands need the `project` scope. If one fails and asks for it, run `gh auth refresh -s project` and try again.

## Find the Project

```bash
gh api graphql -F owner='{owner}' -F name='{repo}' -f query='
query($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    nameWithOwner
    projectsV2(first: 100) { nodes { number title url closed owner { ... on User { login } ... on Organization { login } } } }
  }
  repositoryOwner(login: $owner) {
    ... on ProjectV2Owner {
      projectsV2(first: 100) { nodes { number title url closed owner { ... on User { login } ... on Organization { login } } repositories(first: 100) { nodes { nameWithOwner } } } }
    }
  }
}' --jq '.data.repository.nameWithOwner as $repo
  | {repo: $repo, projects: ([.data.repository.projectsV2.nodes[], (.data.repositoryOwner.projectsV2.nodes[] | select(any(.repositories.nodes[]; .nameWithOwner == $repo)))]
      | map(select(.closed | not) | {number, title, url, owner: .owner.login}) | unique_by(.owner, .number))}'
```

The query lists the Projects linked to the repository, and the owner's Projects that link back to it. A user-owned Project can be missing from the repository's own listing for a while after it is linked, so neither list alone is enough.

`repo` is the repository, as `owner/name`. Each project has the `number` and `owner` that every `gh project` command below takes.

- **One Project:** go to [Check](#check).
- **None:** go to [Create](#create).
- **Two or more:** list each one's title and URL. The skills need exactly one, so ask the user which to keep and to unlink the others with `gh project unlink <n> --owner <owner> --repo <repo>`. Stop. Never pick one yourself.

## Create

First find the owner and the title:

```bash
gh repo view --json name,owner
```

The title is the repository's `name`, and `<owner>` is `owner.login`, for a user and an organization alike. Never use `@me`: `gh project link` rejects it when `--repo` names the owner. A Project can only link to a repository with the same owner, so for a user's repository that user must be you. If it belongs to someone else, stop and say that its owner has to run this skill.

Then create the Project, add the three fields and link it:

```bash
gh project create --owner <owner> --title "<repo name>" --format json --jq .number
gh project field-create <n> --owner <owner> --name Kind --data-type SINGLE_SELECT --single-select-options Epic,Story,Bug,Chore
gh project field-create <n> --owner <owner> --name Priority --data-type SINGLE_SELECT --single-select-options Must,Should,Could,Wont
gh project field-create <n> --owner <owner> --name Size --data-type SINGLE_SELECT --single-select-options XS,S,M,L,XL
gh project link <n> --owner <owner> --repo <repo>
```

The first command prints the Project's number, `<n>`. Stop at the first error and show it.

A new Project already has the Status field with Todo, In Progress and Done. It also has two workflows on: item closed sets Done, and pull request merged sets Done. Nothing else needs setting.

Print the Project's URL with `gh project view <n> --owner <owner> --format json --jq .url`, then go to [Check](#check).

## Check

List the Project's fields and their options:

```bash
gh project field-list <n> --owner <owner> --format json --jq '.fields[] | select(.options) | "\(.name): \(.options | map(.name) | join(", "))"'
```

Compare the output with the contract. Names must match exactly, including case: `In progress` is not `In Progress`.

| Field | Options the skills use |
|---|---|
| Kind | Epic, Story. Bug and Chore are needed only to shape bugs and chores. |
| Priority | Must, Should, Could, Wont |
| Size | XS, S, M, L, XL |
| Status | Todo, In Progress, Done |

Extra fields and extra options are fine. The skills ignore them.

Tell the user each gap and its fix. Run a fix only when the user says so.

- **A missing field:** run its `field-create` command from [Create](#create).
- **A field that lacks an option:** `gh` cannot add an option to a field. Delete the field and create it again. Get its id with `gh project field-list <n> --owner <owner> --format json --jq '.fields[] | select(.name == "<field>") | .id'`, then run `gh project field-delete --id <field id>` and the field's `field-create` command. Warn the user first: the delete clears that field on every item, so they set the values again by hand.
- **A missing Status option:** `gh` cannot change Status. Give the user the Project's settings page, `<project url>/settings`, and say which option to add or rename. A rename keeps every item's value.

When there are no gaps, say the backlog is ready. Then tell the user two things:

- New issues join the Project when they are created with `--project`. `lets-shape` does that.
- Auto-add, in the Project's workflows, is optional. Turn it on to add issues created outside the skills.
