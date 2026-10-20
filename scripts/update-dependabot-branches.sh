#!/usr/bin/env bash
# Update only approved Dependabot PRs; the App token makes CI run on each update.
set -euo pipefail

: "${GH_REPO:?Set GH_REPO to the repository owner/name}"

if [[ $# -gt 0 ]]; then
  [[ "$1" =~ ^[0-9]+$ ]]
  numbers=("$1")
else
  candidates=$(gh api "repos/$GH_REPO/pulls?state=open&base=main&per_page=100" \
    --paginate --jq '.[] | select(.user.login == "dependabot[bot]") | .number')
  mapfile -t numbers <<< "$candidates"
fi

failed=0
for number in "${numbers[@]}"; do
  [[ -n "$number" ]] || continue
  pr=$(gh api "repos/$GH_REPO/pulls/$number")
  if ! jq -e --arg repo "$GH_REPO" '
    .state == "open" and (.draft | not) and .base.ref == "main"
    and .user.login == "dependabot[bot]" and .head.repo.full_name == $repo
    and .auto_merge != null' <<< "$pr" > /dev/null; then
    continue
  fi
  head_sha=$(jq -r '.head.sha' <<< "$pr")
  base_sha=$(jq -r '.base.sha' <<< "$pr")
  behind=$(gh api "repos/$GH_REPO/compare/$base_sha...$head_sha" --jq '.behind_by')
  if [[ "$behind" -gt 0 ]]; then
    if ! gh api --method PUT "repos/$GH_REPO/pulls/$number/update-branch" \
      -f "expected_head_sha=$head_sha"; then
      echo "Could not update PR #$number; continuing with other PRs." >&2
      failed=1
    fi
  fi
done
exit "$failed"
