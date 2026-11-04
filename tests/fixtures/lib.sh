# Shared setup for the fixture scripts. Sourced, not run.
#
# Each script takes one argument, the directory to create the repository in,
# and builds it with fixed names, emails and dates so results are reproducible.

set -euo pipefail

export GIT_AUTHOR_NAME="Ada Fixture"
export GIT_AUTHOR_EMAIL="ada@example.com"
export GIT_COMMITTER_NAME="Ada Fixture"
export GIT_COMMITTER_EMAIL="ada@example.com"
export GIT_CONFIG_NOSYSTEM=1
export GIT_CONFIG_GLOBAL=/dev/null
export TZ=UTC
export LC_ALL=C

_tick=0

# Moves the clock one hour forward, for the next commit or tag.
tick() {
    _tick=$((_tick + 1))
    export GIT_AUTHOR_DATE="$((1700000000 + _tick * 3600)) +0000"
    export GIT_COMMITTER_DATE="$GIT_AUTHOR_DATE"
}

# Commits everything that's staged, with the given message.
commit() {
    tick
    git commit -q --allow-empty -m "$1"
}

# Creates a repository in $1 and changes into it.
init_repo() {
    git init -q -b main "$1"
    cd "$1"
}

# Prints lines "<prefix> 1" to "<prefix> <count>".
lines() {
    local prefix=$1 count=$2 i
    for ((i = 1; i <= count; i++)); do
        echo "$prefix $i"
    done
}

# Edits a file in place with sed expressions. Works with GNU and BSD sed, and
# keeps the file's mode.
edit() {
    local file=$1
    shift
    sed "$@" "$file" > "$file.tmp"
    cat "$file.tmp" > "$file"
    rm "$file.tmp"
}

# Prints standard input with CRLF line endings.
crlf() {
    awk '{ printf "%s\r\n", $0 }'
}
