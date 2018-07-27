#!/bin/bash

set -e

KBD_TYPE_FILE=~/.config/scode/dotfiles/kbdtype
if [[ -e $KBD_TYPE_FILE ]]; then
    KBD_TYPE="$(cat $KBD_TYPE_FILE)"
else
    KBD_TYPE=generic
fi

xkbcomp -w0 "-I${HOME}/.xkb" "${HOME}/.xkb/$KBD_TYPE.xkb" "${DISPLAY}"
