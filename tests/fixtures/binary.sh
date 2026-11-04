#!/usr/bin/env bash
# Binary files, mode changes, submodule updates, deleted files and a file that
# isn't valid UTF-8.
source "$(dirname "$0")/lib.sh"
upstream="$(dirname "$1")/upstream"

# The repository used as a submodule, with three commits.
init_repo "$upstream"
for n in 1 2 3; do
    echo "version $n" > lib.txt
    git add lib.txt
    commit "Library v$n"
done
git tag v1 HEAD~2
git tag v2 HEAD~1
cd - > /dev/null

init_repo "$1"
printf 'PNG\0\1\2\3header' > image.png
lines "echo" 3 > script.sh
lines "tool" 3 > tool.sh
lines "gone" 4 > gone.txt
printf 'caf\xe9\nna\xefve\n' > latin1.txt
git add -A
git -c protocol.file.allow=always submodule --quiet add "$upstream" sub
git -C sub checkout -q v1
git add sub
commit "Add files and submodule"

printf 'PNG\0\4\5\6header-changed' > image.png
chmod +x script.sh
chmod +x tool.sh
echo "tool 4" >> tool.sh
git rm -q gone.txt
printf 'data\0\0\0' > new.bin
printf 'caf\xe9\nna\xefve\nr\xe9sum\xe9\n' > latin1.txt
git -C sub checkout -q v2
git add -A
commit "Change files and update submodule"

# Work-tree changes: binary content, a mode change back, and the submodule moved on.
printf 'PNG\0\7\7\7header-again' > image.png
chmod -x script.sh
git -C sub checkout -q main
