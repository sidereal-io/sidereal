#!/bin/sh
# Apply the weekly mise tool update to the working tree, but only when it
# changes files that an update may change. The publish job of
# mise-tool-updates.yml runs it on the patch from the update job. That job
# ran newly released tools, so the patch is untrusted: only git reads it,
# and every path is checked before anything is applied.
#
#   scripts/apply-mise-tool-update.sh PATCH
set -eu
[ $# -eq 1 ] || { echo "usage: apply-mise-tool-update.sh PATCH" >&2; exit 2; }
patch=$1
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
cd "$root"

# The update job leaves an empty patch when nothing changed.
[ -s "$patch" ] || exit 0

refused=0
# The fixed prefix keeps a crafted path from starting a log line, where the
# runner would read it as a workflow command.
refuse() {
  printf 'apply-mise-tool-update: refused: %s\n' "$1"
  refused=1
}

# mise upgrade and mise lock change mise.toml and mise.lock; lets sync
# changes the rest. A path Git prints quoted, because it holds a control
# character, matches none of these.
allowed() {
  case $1 in
    ../* | */../* | */..) return 1 ;;
    mise.toml | mise.lock | AGENTS.md) return 0 ;;
    .agents/skills/?* | openspec/schemas/?*) return 0 ;;
  esac
  return 1
}

# Listing with git apply itself means the paths checked are the paths it
# would write. A file that holds no patch makes git fail here.
summary=$(git -c core.quotePath=false apply --summary "$patch")
numstat=$(git -c core.quotePath=false apply --numstat "$patch")

# The update job makes no renames, and an update writes only regular files.
while IFS= read -r line; do
  case $line in
    ' rename '* | ' copy '*) refuse "a rename or copy:$line" ;;
    ' create mode '* | ' mode change '*)
      rest=${line#' create mode '}
      rest=${rest#' mode change '* => }
      mode=${rest%% *}
      case $mode in
        100644 | 100755) ;;
        *) refuse "${rest#* } (mode $mode)" ;;
      esac
      ;;
  esac
done <<END
$summary
END

tab=$(printf '\t')
while IFS= read -r line; do
  [ -n "$line" ] || continue
  path=${line#*"$tab"*"$tab"}
  allowed "$path" || refuse "$path"
done <<END
$numstat
END

if [ "$refused" -ne 0 ]; then
  echo "apply-mise-tool-update: the update changed files it may not change, so nothing was applied" >&2
  exit 1
fi
git apply "$patch"
