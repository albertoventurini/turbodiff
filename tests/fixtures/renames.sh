#!/usr/bin/env bash
# Renames with and without content changes, near the similarity threshold, and
# staged and unstaged renames in the work tree.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

lines "pure" 20 > pure.txt
lines "edited" 20 > edited.txt
lines "near" 20 > near.txt
lines "far" 20 > far.txt
lines "staged" 20 > staged.txt
lines "unstaged" 20 > unstaged.txt
git add -A
commit "Add files"

# A rename without changes.
mkdir moved
git mv pure.txt moved/pure.txt
# A rename with a small change (about 90% similar).
git mv edited.txt edited-renamed.txt
edit edited-renamed.txt -e 's/^edited 3$/edited three/' -e 's/^edited 15$/edited fifteen/'
# About 55% similar, just above git's 50% threshold: still a rename.
git mv near.txt near-renamed.txt
edit near-renamed.txt -E 's/^near (1|3|5|7|9|11|13|15|17)$/NEAR \1/'
# About 48% similar, just below the threshold: a deletion and an addition.
git mv far.txt far-renamed.txt
edit far-renamed.txt -E 's/^far ([1-9]|10|11)$/FAR \1/'
git add -A
commit "Rename files"

# A staged rename.
git mv staged.txt staged-renamed.txt
# An unstaged rename: the old file is deleted, the new one is untracked.
mv unstaged.txt unstaged-renamed.txt
edit unstaged-renamed.txt 's/^unstaged 2$/unstaged two/'
