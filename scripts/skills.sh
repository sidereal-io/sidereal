#!/usr/bin/env bash
# Generate the OpenSpec agent skills into .agents/skills from the settings in
# .config/openspec/config.json. Run it through `just skills`.
#
# With --if-stale, do nothing when the stamp from the last successful run
# still matches the current inputs. The `enter` recipe uses this on shell load.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd -P)"
cd "$root"

if_stale=0
case "${1:-}" in
  "") ;;
  --if-stale) if_stale=1 ;;
  *) echo "usage: scripts/skills.sh [--if-stale]" >&2
     exit 2 ;;
esac

stamp=.agents/skills/.openspec-stamp

# Print the inputs the skills are made from: the CLI version and the Git hash
# of each file that decides what gets generated.
current_inputs() {
  local cli config script
  cli="$(OPENSPEC_TELEMETRY=0 openspec --version)" &&
    config="$(git hash-object .config/openspec/config.json)" &&
    script="$(git hash-object scripts/skills.sh)" || return 1
  printf 'cli %s\nconfig %s\nscript %s\n' "$cli" "$config" "$script"
}

# Succeed when the stamp matches the current inputs and every skill folder it
# lists still holds a SKILL.md.
skills_are_fresh() {
  local expected kind name
  [ -f "$stamp" ] || return 1
  expected="$(current_inputs)" || return 1
  [ "$(grep -v '^skill ' "$stamp")" = "$expected" ] || return 1
  grep -q '^skill ' "$stamp" || return 1
  while read -r kind name; do
    [ "$kind" = skill ] || continue
    case "$name" in */*) return 1 ;; openspec-?*) ;; *) return 1 ;; esac
    [ -f ".agents/skills/$name/SKILL.md" ] || return 1
  done <"$stamp"
}

# 1. Stop before touching any file unless the CLI is installed.
if ! command -v openspec >/dev/null; then
  echo "error: just skills needs the openspec CLI. The Nix shell provides it." >&2
  echo "Without Nix, install it with: npm install -g @fission-ai/openspec" >&2
  exit 1
fi

# 2. Refuse to delete through a link that leaves the repo.
for dir in .agents .agents/skills; do
  [ -e "$dir" ] || [ -L "$dir" ] || continue
  real="$(cd "$dir" 2>/dev/null && pwd -P)" || real=""
  case "$real" in
    "$root"/*) ;;
    *) echo "error: $dir resolves to ${real:-a missing directory}, outside $root. Nothing was changed." >&2
       exit 1 ;;
  esac
done

# 3. With --if-stale, leave current skills alone.
if [ "$if_stale" = 1 ] && skills_are_fresh; then
  exit 0
fi

trap 'echo "error: a step failed, so the generated skills may be missing. Run \`just skills\` again, or update openspec if the CLI failed." >&2' ERR

# 4. Start from the same state on every machine: no generated skills, no
# marker, no stamp. A failed run therefore leaves no stamp, and the next
# --if-stale run retries.
mkdir -p .agents/skills
rm -rf .agents/skills/openspec-* .agents/skills/.openspec-target .agents/skills/.openspec-stamp*

# 5. Generate. The CLI reads profile, delivery and workflows only from its global
# config, so point XDG_CONFIG_HOME at the repo for this one command. Remove the
# redirect once the CLI reads those settings from the project:
# https://github.com/Fission-AI/OpenSpec/issues/914
# https://github.com/Fission-AI/OpenSpec/issues/779
XDG_CONFIG_HOME="$root/.config" OPENSPEC_TELEMETRY=0 \
  openspec init --tools agents --no-animation

# 6. Record what the skills were made from. Write to a temporary file and
# rename it, so a stamp is never half written.
tmp="$(mktemp "$stamp.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
{
  current_inputs
  for dir in .agents/skills/openspec-*/; do
    [ -d "$dir" ] || continue
    printf 'skill %s\n' "$(basename "$dir")"
  done
} >"$tmp"
mv "$tmp" "$stamp"
