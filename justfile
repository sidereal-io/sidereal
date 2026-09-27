# Sidereal — single command front door spanning the Rust backend and the
# Vite frontend. See https://github.com/casey/just and backend/README.md.

# OpenSpec CLI version `just skills` requires. The Nix shell pins the CLI through
# flake.lock; when a lock bump moves it, update this value to match.
openspec_version := "1.13.1"

# OpenSpec workflows that `just skills` generates as skills.
openspec_workflows := "propose explore continue apply update sync archive verify"

# Show available recipes.
default:
    @just --list

# Generate the OpenSpec agent skills into .agents/skills from the settings above.
skills:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$(cd "{{justfile_directory()}}" && pwd -P)"
    cd "$root"

    # 1. Stop before touching any file unless the pinned CLI is installed.
    installed="$(openspec --version 2>/dev/null || true)"
    if [ "$installed" != "{{openspec_version}}" ]; then
      echo "error: just skills needs openspec {{openspec_version}}, found ${installed:-no openspec on PATH}." >&2
      echo "Install it with: npm install -g @fission-ai/openspec@{{openspec_version}}" >&2
      exit 1
    fi

    # 2. Refuse to delete or write through a link that leaves the repo.
    for dir in .agents .agents/skills .config; do
      [ -e "$dir" ] || [ -L "$dir" ] || continue
      real="$(cd "$dir" 2>/dev/null && pwd -P)" || real=""
      case "$real" in
        "$root"/*) ;;
        *) echo "error: $dir resolves to ${real:-a missing directory}, outside $root. Nothing was changed." >&2
           exit 1 ;;
      esac
    done

    trap 'echo "error: a step failed, so the generated skills may be missing. Run \`just skills\` again." >&2' ERR

    # 3. Start from the same state on every machine: no generated skills, no marker.
    mkdir -p .agents/skills .config
    rm -rf .agents/skills/openspec-* .agents/skills/.openspec-target

    # 4. Write the repo's OpenSpec settings into a fresh .config/openspec.
    rm -rf .config/openspec
    mkdir .config/openspec
    workflows="$(printf '"%s", ' {{openspec_workflows}})"
    printf '{\n  "profile": "custom",\n  "delivery": "skills",\n  "workflows": [%s]\n}\n' \
      "${workflows%, }" > .config/openspec/config.json

    # 5. Generate. The CLI reads profile, delivery and workflows only from its global
    # config, so point XDG_CONFIG_HOME at the repo for this one command. Remove the
    # redirect once the CLI reads those settings from the project:
    # https://github.com/Fission-AI/OpenSpec/issues/914
    # https://github.com/Fission-AI/OpenSpec/issues/779
    XDG_CONFIG_HOME="$root/.config" OPENSPEC_TELEMETRY=0 \
      openspec init --tools agents --no-animation

# Zero-to-running: backend + frontend together.
dev:
    npx concurrently -n backend,frontend -c blue,green \
      "cargo run -p sidereal-server --manifest-path backend/Cargo.toml" \
      "npm run dev:frontend"

# Run the Rust backend only (serves GET /healthz).
backend:
    cargo run -p sidereal-server --manifest-path backend/Cargo.toml

# Run the Vite frontend only.
frontend:
    npm run dev:frontend

# Backend checks: format, lint (deny warnings), tests, dependency-direction lint.
check:
    cd backend && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    backend/scripts/check-arch.sh
