# The project board's Status rules (issue #404). A pure function: it reads
# facts and prints the Status each board item should have. It writes nothing.
#
# Input: the pages that scripts/board-sync.sh fetches with
# `gh api graphql --paginate --slurp`, an array of {data: {organization:
# {projectV2: {items: {nodes}}}}}.
# Output: one object per issue on the board: {item, number, have, want}.

def issue_items:
  [.[].data.organization.projectV2.items.nodes[]
   | select(.isArchived | not)
   | select(.content.__typename == "Issue")];

# The Epics that have an open, assigned sub-issue, as a set of issue URLs.
def claimed_epic_set:
  map(.content
      | select(.state == "OPEN" and .assignees.totalCount > 0 and .parent != null)
      | {key: .parent.url, value: true})
  | from_entries;

# An Epic: the first rule that matches wins.
def epic_status($claimed):
  if .state == "CLOSED" then "Done"
  elif .subIssuesSummary.completed > 0 or $claimed[.url] then "In progress"
  elif .issueDependenciesSummary.blockedBy > 0 then "Blocked"
  elif .subIssuesSummary.total == 0 then "Backlog"
  else "Ready"
  end;

# A Story, Bug, Chore, or issue with no type: the first rule that matches wins.
def work_status:
  if .state == "CLOSED" then "Done"
  elif any(.closedByPullRequestsReferences.nodes[]; .state == "OPEN" and (.isDraft | not)) then "In review"
  elif .assignees.totalCount > 0 then "In progress"
  elif .issueDependenciesSummary.blockedBy > 0 then "Blocked"
  else "Ready"
  end;

issue_items
| claimed_epic_set as $claimed
| .[]
| {
    item: .id,
    number: .content.number,
    have: .status.name,
    want: (.content | if .issueType.name == "Epic" then epic_status($claimed) else work_status end)
  }
