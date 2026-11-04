#!/usr/bin/env bash
# Line-ending conversion and `ident` through .gitattributes.
source "$(dirname "$0")/lib.sh"
init_repo "$1"

printf '*.txt text\n*.c ident\n' > .gitattributes
lines "same" 5 > same.txt
lines "changed" 5 > changed.txt
printf '/* $Id$ */\nint x;\n' > ident.c
printf '/* $Id$ */\nint y;\n' > ident-changed.c
git add -A
commit "Add files"

# Only the line endings differ: not a change after conversion.
lines "same" 5 | crlf > same.txt
# Line endings and one line differ.
lines "changed" 5 | sed -e 's/^changed 2$/changed two/' | crlf > changed.txt
# An expanded `$Id$` keyword: not a change after the ident filter.
printf '/* $Id: 0123456789abcdef0123456789abcdef01234567 $ */\nint x;\n' > ident.c
printf '/* $Id: 0123456789abcdef0123456789abcdef01234567 $ */\nint z;\n' > ident-changed.c
