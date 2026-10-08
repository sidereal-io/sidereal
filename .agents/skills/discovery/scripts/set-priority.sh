#!/usr/bin/env bash
# shellcheck disable=SC2016 # $vars in the single-quoted queries are GraphQL variables.
# Set an issue's MoSCoW priority in the organization's Priority issue field.
# gh cannot set issue fields yet, so this uses GraphQL.
# Usage: set-priority.sh <issue-number> <Must|Should|Could|Wont>
# Runs against the current repository. Needs gh.
set -euo pipefail

n="${1:?usage: set-priority.sh <issue-number> <Must|Should|Could|Wont>}"
priority="${2:?usage: set-priority.sh <issue-number> <Must|Should|Could|Wont>}"

repo=$(gh repo view --json owner,name -q '.owner.login + "/" + .name')
issue_id=$(gh issue view "$n" --json id -q .id)

read -r field_id option_id < <(gh api graphql -F org="${repo%/*}" -f query='
query($org: String!) {
  organization(login: $org) {
    issueFields(first: 50) { nodes { ... on IssueFieldSingleSelect { id name options { id name } } } }
  }
}' --jq ".data.organization.issueFields.nodes[] | select(.name == \"Priority\")
  | .id + \" \" + ([.options[] | select(.name == \"$priority\") | .id][0] // \"\")") || true

if [[ -z "${field_id:-}" ]]; then
  echo "The ${repo%/*} organization has no Priority issue field." >&2
  exit 1
fi
if [[ -z "${option_id:-}" ]]; then
  echo "Priority has no option '$priority'. Use Must, Should, Could, or Wont." >&2
  exit 1
fi

gh api graphql -F issue="$issue_id" -F field="$field_id" -F option="$option_id" -f query='
mutation($issue: ID!, $field: ID!, $option: ID!) {
  setIssueFieldValue(input: {issueId: $issue, issueFields: [{fieldId: $field, singleSelectOptionId: $option}]}) {
    clientMutationId
  }
}' >/dev/null
echo "#$n Priority: $priority"
