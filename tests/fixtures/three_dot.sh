#!/usr/bin/env bash
# Three-dot vs two-dot where base has moved on: `billing.py` is added to main
# after feature branched off (PRD §6.3). HEAD is feature, with work-tree edits.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

lines "app" 10 > app.py
git add -A
commit "Add app"

git switch -q -c feature
edit app.py 's/^app 5$/app five/'
lines "feature" 5 > feature.py
git add -A
commit "Add feature"

git switch -q main
lines "billing" 6 > billing.py
git add -A
commit "Add billing"

git switch -q feature
echo "feature 6" >> feature.py
