#!/usr/bin/env bash
# Line-ending conversion and `ident` through .gitattributes.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

printf '*.txt text\n*.c ident\n*.dat filter=probe\n' > .gitattributes
lines "same" 5 > same.txt
lines "changed" 5 > changed.txt
printf '/* $Id$ */\nint x;\n' > ident.c
printf '/* $Id$ */\nint y;\n' > ident-changed.c
echo "data 1" > driver.dat
git add -A
commit "Add files"

# An external filter driver, set up after committing. TurboDiff must never run
# one, since drivers such as Git LFS can write into .git. This one leaves a
# marker file if it runs, and otherwise changes nothing.
git config filter.probe.clean "touch '$PWD/.git/filter-ran'; cat"
git config filter.probe.smudge cat
echo "data 2" > driver.dat

# Only the line endings differ: not a change after conversion.
lines "same" 5 | crlf > same.txt
# Line endings and one line differ.
lines "changed" 5 | sed -e 's/^changed 2$/changed two/' | crlf > changed.txt
# An expanded `$Id$` keyword: not a change after the ident filter.
printf '/* $Id: 0123456789abcdef0123456789abcdef01234567 $ */\nint x;\n' > ident.c
printf '/* $Id: 0123456789abcdef0123456789abcdef01234567 $ */\nint z;\n' > ident-changed.c
