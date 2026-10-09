#!/usr/bin/env bash
# Set each issue's Status on the project board from the issue's own state:
# closed, linked pull request, assignees, blockers, and sub-issues. The rules
# live in scripts/board-status.jq. Run it through `just board-sync`.
#
# With --dry-run, print the changes and write nothing. Needs gh, logged in with
# the `project` scope (`gh auth refresh -s project`), and jq.
#
# The reads use GraphQL: the REST items endpoint returns HTTP 500 for every
# page past the board's first 100 items.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd -P)"
cd "$root"

org=sidereal-io
project=3

dry_run=0
case "${1:-}" in
  "") ;;
  --dry-run) dry_run=1 ;;
  *) echo "usage: scripts/board-sync.sh [--dry-run]" >&2
     exit 2 ;;
esac

# shellcheck disable=SC2016 # $endCursor and $org are GraphQL variables.
query='
query($org: String!, $project: Int!, $endCursor: String) {
  organization(login: $org) {
    projectV2(number: $project) {
      id
      field(name: "Status") { ... on ProjectV2SingleSelectField { id options { id name } } }
      items(first: 100, after: $endCursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id
          isArchived
          status: fieldValueByName(name: "Status") { ... on ProjectV2ItemFieldSingleSelectValue { name } }
          content {
            __typename
            ... on Issue {
              number url state
              issueType { name }
              assignees { totalCount }
              parent { url }
              subIssuesSummary { total completed }
              issueDependenciesSummary { blockedBy }
              closedByPullRequestsReferences(first: 10, includeClosedPrs: false) { nodes { isDraft state } }
            }
          }
        }
      }
    }
  }
}'

pages="$(gh api graphql --paginate --slurp -F org="$org" -F project="$project" -f query="$query")"
project_id="$(jq -r '.[0].data.organization.projectV2.id' <<<"$pages")"
field="$(jq -c '.[0].data.organization.projectV2.field' <<<"$pages")"
field_id="$(jq -r '.id' <<<"$field")"
changes="$(jq -c -f scripts/board-status.jq <<<"$pages" | jq -c 'select(.have != .want)')"

if [ -z "$changes" ]; then
  echo "Board is in sync."
  exit 0
fi

while read -r change; do
  item="$(jq -r '.item' <<<"$change")"
  number="$(jq -r '.number' <<<"$change")"
  have="$(jq -r '.have // "(none)"' <<<"$change")"
  want="$(jq -r '.want' <<<"$change")"
  echo "#$number: $have -> $want"
  [ "$dry_run" -eq 1 ] && continue

  option_id="$(jq -r --arg want "$want" '.options[] | select(.name == $want) | .id' <<<"$field")"
  if [ -z "$option_id" ]; then
    echo "error: the board has no Status option named \"$want\"" >&2
    exit 1
  fi
  gh project item-edit --project-id "$project_id" --id "$item" \
    --field-id "$field_id" --single-select-option-id "$option_id" >/dev/null
done <<<"$changes"

if [ "$dry_run" -eq 1 ]; then
  echo "Dry run: nothing written."
fi
