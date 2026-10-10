#!/usr/bin/env bash
# Check scripts/board-status.jq against one board item per rule. Each fixture
# item carries the Status it should get as `expect`. Run it through
# `just board-sync-test`.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd -P)"
cd "$root"

# Build one board item. Arguments: number, type, state, assignees, open
# blockers, sub-issues total, sub-issues completed, parent number or "",
# pull request ("ready", "draft", or ""), expected Status.
item() {
  jq -n --argjson n "$1" --arg type "$2" --arg state "$3" --argjson assignees "$4" \
    --argjson blockers "$5" --argjson subs "$6" --argjson done "$7" --arg parent "$8" \
    --arg pr "$9" --arg expect "${10}" '{
      id: "item-\($n)", isArchived: false, status: null, expect: $expect,
      content: {
        __typename: "Issue", number: $n, url: "issue-\($n)", state: $state,
        issueType: (if $type == "" then null else {name: $type} end),
        assignees: {totalCount: $assignees},
        parent: (if $parent == "" then null else {url: "issue-\($parent)"} end),
        subIssuesSummary: {total: $subs, completed: $done},
        issueDependenciesSummary: {blockedBy: $blockers},
        closedByPullRequestsReferences: {nodes: (
          if $pr == "" then [] else [{state: "OPEN", isDraft: ($pr == "draft")}] end)}
      }}'
}

fixture="$(
  {
    #    n   type    state   asg blk subs done parent pr      expect
    item 1  Story   CLOSED  1   1   0    0    ""     ""      "Done"
    item 2  Story   OPEN    1   1   0    0    ""     ready   "In review"
    item 3  Story   OPEN    1   1   0    0    ""     draft   "In progress"
    item 4  Bug     OPEN    0   2   0    0    ""     draft   "Blocked"
    item 5  Chore   OPEN    0   0   0    0    ""     ""      "Ready"
    item 6  ""      OPEN    0   0   0    0    ""     ""      "Ready"
    item 10 Epic    CLOSED  0   1   0    0    ""     ""      "Done"
    item 11 Epic    OPEN    0   1   3    1    ""     ""      "In progress"
    item 12 Epic    OPEN    0   1   3    0    ""     ""      "In progress"
    item 13 Story   OPEN    1   0   0    0    12     ""      "In progress"
    item 14 Epic    OPEN    0   1   0    0    ""     ""      "Blocked"
    item 15 Epic    OPEN    0   0   0    0    ""     ""      "Backlog"
    item 16 Epic    OPEN    0   0   2    0    ""     ""      "Ready"
    item 17 Story   OPEN    0   0   0    0    16     ""      "Ready"
  } | jq -s '[{data: {organization: {projectV2: {items: {nodes: .}}}}}]'
)"

# Add one item the rules must skip: content this login cannot see.
fixture="$(jq '.[0].data.organization.projectV2.items.nodes += [{id: "hidden", isArchived: false, status: null, content: null}]' <<<"$fixture")"

expected="$(jq -c '.[0].data.organization.projectV2.items.nodes[] | select(.content) | {number: .content.number, want: .expect}' <<<"$fixture")"
actual="$(jq -c -f scripts/board-status.jq <<<"$fixture" | jq -c '{number, want}')"

if [ "$expected" = "$actual" ]; then
  echo "board-status rules: all $(wc -l <<<"$expected") cases pass."
else
  echo "board-status rules: mismatch (< expected, > actual)" >&2
  diff <(echo "$expected") <(echo "$actual") >&2 || true
  exit 1
fi
