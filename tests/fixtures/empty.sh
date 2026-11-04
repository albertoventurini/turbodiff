#!/usr/bin/env bash
# A repository with no commits yet: one staged file, untracked files, an ignored one.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

echo "*.log" > .gitignore
lines "staged" 3 > staged.txt
git add staged.txt
lines "untracked" 2 > untracked.txt
mkdir docs
lines "readme" 4 > docs/readme.md
echo "ignored" > ignored.log
