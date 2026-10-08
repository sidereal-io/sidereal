#!/usr/bin/env bash
# shellcheck disable=SC2016 # $vars in the single-quoted queries are GraphQL variables.
# Print the open backlog: one JSON object per open issue, with the facts needed
# to pick the next story. gh cannot read issue fields yet, so this uses GraphQL.
# Usage: scripts/backlog.sh
# Each line: {"number","title","type","priority","assignees","subIssues","parent","openBlockers"}
# "priority" is the Priority issue field (Must, Should, Could, Wont) or null.
# "openBlockers" lists only the issue's own blockers, not its ancestors'. Needs gh.
set -euo pipefail

repo=$(gh repo view --json owner,name -q '.owner.login + "/" + .name')

gh api graphql --paginate -F owner="${repo%/*}" -F name="${repo#*/}" -f query='
query($owner: String!, $name: String!, $endCursor: String) {
  repository(owner: $owner, name: $name) {
    issues(states: OPEN, first: 100, after: $endCursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number title
        issueType { name }
        assignees(first: 10) { nodes { login } }
        subIssues { totalCount }
        parent { number }
        blockedBy(first: 50) { nodes { number state } }
        issueFieldValues(first: 20) {
          nodes { ... on IssueFieldSingleSelectValue { name field { ... on IssueFieldSingleSelect { name } } } }
        }
      }
    }
  }
}' --jq '.data.repository.issues.nodes[] | {
  number, title,
  type: .issueType.name,
  priority: ([.issueFieldValues.nodes[] | select(.field.name == "Priority") | .name][0]),
  assignees: [.assignees.nodes[].login],
  subIssues: .subIssues.totalCount,
  parent: .parent.number,
  openBlockers: [.blockedBy.nodes[] | select(.state == "OPEN") | .number]
}'
