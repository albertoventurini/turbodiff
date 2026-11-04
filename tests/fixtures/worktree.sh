#!/usr/bin/env bash
# Staged, unstaged and untracked changes, ignored files and deleted files.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

printf 'build/\n*.log\n' > .gitignore
lines "modified" 10 > modified.txt
lines "staged" 10 > staged.txt
lines "both" 10 > both.txt
lines "deleted" 5 > deleted.txt
lines "staged-deleted" 5 > staged-deleted.txt
mkdir src
lines "fn" 30 > src/lib.rs
git add -A
commit "Initial commit"

# Unstaged edit.
edit modified.txt 's/^modified 3$/modified three/'
# Staged edit.
edit staged.txt 's/^staged 5$/staged five/'
git add staged.txt
# Staged edit, then another unstaged one on top.
edit both.txt 's/^both 1$/both one/'
git add both.txt
echo "both 11" >> both.txt
# Unstaged and staged deletions.
rm deleted.txt
git rm -q staged-deleted.txt
# Staged new file.
lines "staged-new" 3 > staged-new.txt
git add staged-new.txt
# Untracked files, including an empty one and one in a new folder.
lines "untracked" 4 > untracked.txt
: > empty.txt
mkdir -p newdir/sub
lines "nested" 2 > newdir/sub/file.txt
# Edits in a subfolder: one deleted line, two inserted.
lines "fn" 30 | awk '$0 != "fn 10" { print } $0 == "fn 20" { print "fn 20a"; print "fn 20b" }' > src/lib.rs
# Ignored, by .gitignore and by info/exclude.
mkdir build
echo "artifact" > build/out.txt
echo "log" > debug.log
echo "secret.txt" >> .git/info/exclude
echo "secret" > secret.txt
