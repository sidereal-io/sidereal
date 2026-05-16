#!/usr/bin/env bash
# PreToolUse hook for Bash. Inspects `gh pr create` invocations to ensure the
# PR body includes `Closes #<n>` matching the current issue branch.
#
# Warn-only: emits a system message but never blocks.
#
# Reads the tool call payload from stdin (JSON with .tool_input.command).

set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
cd "$REPO_ROOT" || exit 0

payload="$(cat)"
command="$(printf '%s' "$payload" | jq -r '.tool_input.command // empty' 2>/dev/null)"
[[ -z "$command" ]] && exit 0

# Only inspect `gh pr create` calls
[[ "$command" != *"gh pr create"* ]] && exit 0

branch="$(git branch --show-current 2>/dev/null)"
if [[ ! "$branch" =~ ^issue-([0-9]+)- ]]; then
  printf '\n[sidereal-workflow] Heads up: creating a PR from \`%s\` (no issue-<n>-* prefix). PR will not auto-close any issue.\n\n' "$branch" >&2
  exit 0
fi

issue_num="${BASH_REMATCH[1]}"

# The body may be inline (`--body "..."`), via heredoc (`--body "$(cat <<EOF ... EOF)"`),
# or via file (`--body-file path`). For warn-only, a substring search across the whole
# command captures the inline + heredoc cases. --body-file we flag separately.
if [[ "$command" == *"--body-file"* ]]; then
  printf '\n[sidereal-workflow] Heads up: PR uses --body-file. Verify it contains \`Closes #%s\` so the issue auto-closes on merge.\n\n' "$issue_num" >&2
  exit 0
fi

# Match `Closes #N`, `closes #N`, `Fixes #N`, `Resolves #N` — all of GitHub's auto-close keywords
if ! printf '%s' "$command" | grep -qiE '(close[sd]?|fix(es|ed)?|resolve[sd]?)[[:space:]]+#'"$issue_num"'\b'; then
  printf '\n[sidereal-workflow] Heads up (warn-only):\n' >&2
  printf '  - PR body does not contain \`Closes #%s\` — issue will not auto-close on merge.\n' "$issue_num" >&2
  printf '  - Add a line like \`Closes #%s\` to the PR body.\n\n' "$issue_num" >&2
fi

exit 0
