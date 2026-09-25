#!/usr/bin/env bash
# List issue links in a discovery map that point at closed issues.
# Usage: stale-links.sh <path/to/discovery.md>
# Prints one line per closed issue: "<line>: #<n> <title>", in file order.
# Prints nothing when every linked issue is open. Needs gh.
set -euo pipefail

map="${1:?usage: stale-links.sh <path/to/discovery.md>}"

declare -A state title
grep -no 'https://github.com/[^/]*/[^/]*/issues/[0-9]*' "$map" |
while IFS=: read -r line url_scheme url_rest; do
  url="$url_scheme:$url_rest"
  repo=$(sed -E 's#https://github.com/([^/]+/[^/]+)/issues/.*#\1#' <<<"$url")
  n="${url##*/}"
  key="$repo#$n"
  if [[ -z "${state[$key]:-}" ]]; then
    read -r state[$key] title[$key] < <(gh issue view "$n" -R "$repo" --json state,title -q '.state + " " + .title')
  fi
  if [[ "${state[$key]}" == CLOSED ]]; then
    echo "$line: #$n ${title[$key]}"
  fi
done
