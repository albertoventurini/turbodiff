#!/bin/sh
# Builds a repository with a very large change set, for checking performance
# by hand (TECH.md §9): thousands of changed files and tens of
# thousands of changed lines, plus a few files over the large-diff thresholds.
#
# Usage: scripts/make-large-repo.sh <dir> [files]
# <dir> must not exist. [files] defaults to 3000.
set -eu

dir=$1
files=${2:-3000}
if [ -e "$dir" ]; then
  echo "$dir already exists" >&2
  exit 1
fi

mkdir -p "$dir"
cd "$dir"
git init -q -b main
git config user.name 'TurboDiff'
git config user.email 'turbodiff@example.com'

# A 200-line source file; $2 picks which lines differ between versions.
source_file() {
  awk -v seed="$1" -v version="$2" 'BEGIN {
    for (i = 1; i <= 200; i++) {
      if (version > 0 && i % 10 == seed % 10) {
        printf "    let value_%d = compute(%d, \"changed in version %d\");\n", i, i * seed, version
      } else if (i % 25 == 1) {
        printf "fn function_%d_%d() {\n", seed, i
      } else if (i % 25 == 0) {
        print "}"
      } else {
        printf "    let value_%d = compute(%d, \"unchanged\");\n", i, i * seed
      }
    }
  }'
}

i=0
while [ "$i" -lt "$files" ]; do
  mkdir -p "src/module_$((i / 100))"
  source_file "$i" 0 > "src/module_$((i / 100))/file_$i.rs"
  i=$((i + 1))
done
seq 1 50000 | awk '{ printf "package-%d@1.0.%d:\n  resolved \"https://example.com/%d\"\n", $1, $1 % 7, $1 }' > big.lock
git add -A
git commit -q -m 'Base'

# Change every file but one in ten, delete those, and add some new ones.
i=0
while [ "$i" -lt "$files" ]; do
  path="src/module_$((i / 100))/file_$i.rs"
  if [ $((i % 10)) -eq 9 ]; then
    rm "$path"
  else
    source_file "$i" 1 > "$path"
  fi
  i=$((i + 1))
done
i=0
while [ "$i" -lt $((files / 10)) ]; do
  mkdir -p new
  source_file "$i" 2 > "new/added_$i.rs"
  i=$((i + 1))
done
seq 1 50000 | awk '{ printf "package-%d@1.1.%d:\n  resolved \"https://example.com/%d\"\n", $1, $1 % 7, $1 }' > big.lock

echo "Built $dir: $(git status --porcelain | wc -l) changed paths"
