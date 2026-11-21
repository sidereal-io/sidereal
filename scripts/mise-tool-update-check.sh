#!/bin/sh
# Checks scripts/apply-mise-tool-update.sh, which the publish job of
# mise-tool-updates.yml runs on the patch from the update job. Each case
# makes a patch in a scratch Git repository the way the update job does,
# then runs a copy of the script there. Needs git only: no network, no mise.
#
#   sh scripts/mise-tool-update-check.sh [case...]
#
# With no case named, it runs every case.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
all="allowed empty workflow config lookalike rename symlink dotdot garbage"

fx=$(mktemp -d "${TMPDIR:-/tmp}/mise-tool-update-check.XXXXXX")
trap 'rm -rf "$fx"' EXIT
trap 'exit 130' HUP INT TERM
repo=$fx/repo

g() { git -C "$repo" "$@"; }

git init -q "$repo"
g config user.name check
g config user.email check@example.invalid
g config commit.gpgsign false
mkdir -p "$repo/.agents/skills/a" "$repo/openspec/schemas/s" "$repo/.github/workflows" "$repo/scripts"
printf '[tools]\npnpm = "12"\n' >"$repo/mise.toml"
printf '[[tools.pnpm]]\nversion = "12.9.0"\n' >"$repo/mise.lock"
printf '# Agents\n' >"$repo/AGENTS.md"
printf 'skill a\n' >"$repo/.agents/skills/a/SKILL.md"
printf 'schema: s\n' >"$repo/openspec/schemas/s/schema.yaml"
printf 'schema: lets-standard\n' >"$repo/openspec/config.yaml"
printf 'name: ci\n' >"$repo/.github/workflows/ci.yml"
# The script under test stays out of the fixture's commits.
echo '/scripts/' >>"$repo/.git/info/exclude"
g add -A
g commit -qm fixture
if [ -f "$root/scripts/apply-mise-tool-update.sh" ]; then
  cp "$root/scripts/apply-mise-tool-update.sh" "$repo/scripts/"
fi

# make_patch NAME [git diff option]: stage the fixture's changes, save them
# as the update job does, record the tree they make, then reset the fixture.
make_patch() {
  g add -A
  g diff --cached --binary --no-renames ${2:+"$2"} >"$fx/$1.patch"
  g write-tree >"$fx/$1.tree"
  g reset -q --hard
  g clean -qfd
}

# run_apply NAME: run the script on the case's patch; sets $status.
run_apply() {
  if (cd "$repo" && sh scripts/apply-mise-tool-update.sh "$fx/$1.patch") >"$fx/out" 2>&1; then
    status=0
  else
    status=$?
  fi
}

failed=0
fail() {
  echo "FAIL $case: $*"
  sed 's/^/  | /' "$fx/out"
  failed=1
}

clean_tree() { test -z "$(g status --porcelain)"; }

# expect_refused NAME TEXT...: the script exits non-zero, prints a refusal
# line holding each TEXT, and leaves the working tree unchanged.
expect_refused() {
  name=$1
  shift
  run_apply "$name"
  if [ "$status" -eq 0 ]; then
    fail "the script applied a patch it must refuse"
  elif ! clean_tree; then
    fail "the working tree changed"
  else
    for p in "$@"; do
      if ! grep -qF "apply-mise-tool-update: refused: $p" "$fx/out"; then
        fail "the output does not name $p"
        return
      fi
    done
    echo "ok $case"
  fi
}

case_allowed() {
  printf '[tools]\npnpm = "12"\ngh = "2"\n' >"$repo/mise.toml"
  printf '[[tools.pnpm]]\nversion = "12.22.0"\n' >"$repo/mise.lock"
  printf '# Agents\n\nnew rules\n' >"$repo/AGENTS.md"
  mkdir -p "$repo/.agents/skills/b"
  printf '#!/bin/sh\n' >"$repo/.agents/skills/b/run.sh"
  chmod +x "$repo/.agents/skills/b/run.sh"
  rm "$repo/openspec/schemas/s/schema.yaml"
  make_patch allowed
  run_apply allowed
  if [ "$status" -ne 0 ]; then
    fail "exit status $status"
  else
    g add -A
    if [ "$(g write-tree)" != "$(cat "$fx/allowed.tree")" ]; then
      fail "the applied tree differs from the update's tree"
    else
      echo "ok $case"
    fi
  fi
}

case_empty() {
  make_patch empty
  run_apply empty
  if [ "$status" -ne 0 ]; then
    fail "exit status $status"
  elif ! clean_tree; then
    fail "the working tree changed"
  else
    echo "ok $case"
  fi
}

case_workflow() {
  printf '[[tools.pnpm]]\nversion = "12.22.0"\n' >"$repo/mise.lock"
  printf 'name: changed\n' >"$repo/.github/workflows/ci.yml"
  make_patch workflow
  expect_refused workflow .github/workflows/ci.yml
}

case_config() {
  printf 'schema: other\n' >"$repo/openspec/config.yaml"
  make_patch config
  expect_refused config openspec/config.yaml
}

case_lookalike() {
  printf 'x\n' >"$repo/mise.toml.bak"
  mkdir -p "$repo/.agents/skillsx"
  printf 'x\n' >"$repo/.agents/skillsx/f"
  make_patch lookalike
  expect_refused lookalike mise.toml.bak .agents/skillsx/f
}

# The update job never detects renames, so a rename is a crafted patch.
case_rename() {
  g mv .github/workflows/ci.yml .agents/skills/ci.yml
  make_patch rename -M
  expect_refused rename 'a rename or copy'
}

case_symlink() {
  ln -s ../../.github "$repo/.agents/skills/link"
  make_patch symlink
  expect_refused symlink '.agents/skills/link (mode 120000)'
}

case_dotdot() {
  cat >"$fx/dotdot.patch" <<'EOF'
--- a/.agents/skills/../../outside
+++ b/.agents/skills/../../outside
@@ -0,0 +1 @@
+x
EOF
  run_apply dotdot
  if [ "$status" -eq 0 ]; then
    fail "the script applied a patch it must refuse"
  elif [ -e "$repo/outside" ] || ! clean_tree; then
    fail "a file changed"
  else
    echo "ok $case"
  fi
}

case_garbage() {
  printf 'not a patch\n' >"$fx/garbage.patch"
  run_apply garbage
  if [ "$status" -eq 0 ]; then
    fail "the script accepted a file that holds no patch"
  elif ! clean_tree; then
    fail "the working tree changed"
  else
    echo "ok $case"
  fi
}

# shellcheck disable=SC2086 # $all is a fixed list of case names
[ $# -gt 0 ] || set -- $all
for case in "$@"; do
  case " $all " in
    *" $case "*) g reset -q --hard; g clean -qfd; : >"$fx/out"; "case_$case" ;;
    *) echo "mise-tool-update-check: no case named $case" >&2; exit 2 ;;
  esac
done
exit "$failed"
