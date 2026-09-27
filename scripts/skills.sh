#!/usr/bin/env bash
# Generate the OpenSpec agent skills into .agents/skills from the settings in
# .config/openspec/config.json. Run it through `just skills`.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd -P)"
cd "$root"

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

trap 'echo "error: a step failed, so the generated skills may be missing. Run \`just skills\` again, or update openspec if the CLI failed." >&2' ERR

# 3. Start from the same state on every machine: no generated skills, no marker.
mkdir -p .agents/skills
rm -rf .agents/skills/openspec-* .agents/skills/.openspec-target

# 4. Generate. The CLI reads profile, delivery and workflows only from its global
# config, so point XDG_CONFIG_HOME at the repo for this one command. Remove the
# redirect once the CLI reads those settings from the project:
# https://github.com/Fission-AI/OpenSpec/issues/914
# https://github.com/Fission-AI/OpenSpec/issues/779
XDG_CONFIG_HOME="$root/.config" OPENSPEC_TELEMETRY=0 \
  openspec init --tools agents --no-animation
