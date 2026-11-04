#!/usr/bin/env bash
# Branches (local and remote-tracking), lightweight and annotated tags, and a
# long history for paging and searching commits.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

for n in $(seq 1 120); do
    if ((n % 10 == 0)); then
        commit "Fix bug number $n"
    else
        commit "Commit $n"
    fi
    case $n in
        10) git tag v0.1 ;;
        20) tick; git tag -a v0.2 -m "Version 0.2" ;;
        30) git branch old ;;
        100) git branch origin-main-base ;;
    esac
done
# An annotated tag on an old commit with the newest tagger date.
tick
git tag -a v0.3 -m "Version 0.3" HEAD~110
git tag v1.0

git switch -q -c feature/login HEAD~5
commit "Login form"
git switch -q main

# Remote-tracking branches, as a fetch would leave them.
git update-ref refs/remotes/origin/main origin-main-base
git branch -q -D origin-main-base
git update-ref refs/remotes/origin/feature/login feature/login
git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main

# Commits that aren't on any branch, made until one shares its first four hex
# digits with another commit, for an ambiguous SHA prefix.
git rev-list --all | cut -c1-4 > .git/prefixes
n=0
while :; do
    n=$((n + 1))
    sha=$(git commit-tree -p HEAD -m "Attempt $n" "HEAD^{tree}")
    if grep -q "^${sha:0:4}\$" .git/prefixes; then
        break
    fi
    echo "${sha:0:4}" >> .git/prefixes
done
rm .git/prefixes
git update-ref refs/fixture/ambiguous "$sha"
