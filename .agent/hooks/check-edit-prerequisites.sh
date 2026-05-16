#!/usr/bin/env bash
# PreToolUse hook for Edit/Write/NotebookEdit.
# Warn-only: emits a system message but never blocks.
#
# Checks (in order; missing prerequisites accumulate into one warning):
#   1. Current branch matches `issue-<n>-*` (not on main, not on a non-issue branch)
#   2. The referenced issue has the `spec/ready` label
#   3. A plan file exists at `.workspace/plans/*issue-<n>*.md`
#
# Reads the tool call payload from stdin; emits warnings to stderr; exits 0 always.

set -uo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
cd "$REPO_ROOT" || exit 0

# Read payload (we don't need to inspect it for path filtering — gating is global)
cat > /dev/null

branch="$(git branch --show-current 2>/dev/null)"
[[ -z "$branch" ]] && exit 0  # detached HEAD or no branch — skip

warnings=()

# --- Check 1: branch name pattern ---
if [[ ! "$branch" =~ ^issue-([0-9]+)- ]]; then
  if [[ "$branch" == "main" || "$branch" == "master" ]]; then
    warnings+=("editing on \`$branch\` — sidereal workflow expects an \`issue-<n>-<slug>\` branch off origin/main")
  else
    warnings+=("branch \`$branch\` does not match \`issue-<n>-<slug>\` — sidereal workflow expects issue-scoped branches")
  fi
  # Without an issue number we can't run checks 2 and 3.
  if [[ ${#warnings[@]} -gt 0 ]]; then
    printf '\n[sidereal-workflow] Heads up:\n' >&2
    for w in "${warnings[@]}"; do printf '  - %s\n' "$w" >&2; done
    printf '  See the sidereal-workflow-overrides skill for the full workflow.\n\n' >&2
  fi
  exit 0
fi

issue_num="${BASH_REMATCH[1]}"

# --- Check 2: spec/ready label (cached per branch) ---
cache_dir=".workspace/cache/label-cache"
mkdir -p "$cache_dir"
cache_file="$cache_dir/$(echo "$branch" | tr '/' '_')"

if [[ -f "$cache_file" ]]; then
  has_label="$(cat "$cache_file")"
else
  if command -v gh >/dev/null 2>&1; then
    labels="$(gh issue view "$issue_num" --json labels --jq '[.labels[].name] | join(",")' 2>/dev/null)"
    if [[ -z "$labels" ]]; then
      has_label="unknown"  # gh failed (network, auth, or issue doesn't exist)
    elif [[ ",$labels," == *",spec/ready,"* ]]; then
      has_label="yes"
    else
      has_label="no:$labels"
    fi
    echo "$has_label" > "$cache_file"
  else
    has_label="unknown"
  fi
fi

case "$has_label" in
  yes)         ;;  # all good
  unknown)     warnings+=("could not verify issue #$issue_num labels (gh failed) — make sure it has \`spec/ready\` before implementing") ;;
  no:*)        warnings+=("issue #$issue_num is missing the \`spec/ready\` label (current labels: ${has_label#no:}) — only humans should apply it") ;;
esac

# --- Check 3: plan file exists ---
shopt -s nullglob
plans=( .workspace/plans/*"issue-${issue_num}"*.md )
shopt -u nullglob
if [[ ${#plans[@]} -eq 0 ]]; then
  warnings+=("no plan file found at \`.workspace/plans/*issue-${issue_num}*.md\` — sidereal workflow expects a plan before implementation")
fi

# --- Emit ---
if [[ ${#warnings[@]} -gt 0 ]]; then
  printf '\n[sidereal-workflow] Heads up (warn-only):\n' >&2
  for w in "${warnings[@]}"; do printf '  - %s\n' "$w" >&2; done
  printf '  See the sidereal-workflow-overrides skill for the full workflow.\n' >&2
  printf '  To clear the label cache: rm -rf .workspace/cache/label-cache\n\n' >&2
fi

exit 0
