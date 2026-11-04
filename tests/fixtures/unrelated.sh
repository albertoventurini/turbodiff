#!/usr/bin/env bash
# Two branches with unrelated histories (no merge base).
source "$(dirname "$0")/lib.sh"
init_repo "$1"

lines "main" 3 > main.txt
git add -A
commit "Main root"

git switch -q --orphan other
lines "other" 3 > other.txt
git add -A
commit "Other root"

git switch -q main
