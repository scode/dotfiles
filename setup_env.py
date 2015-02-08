#!/usr/bin/env python

"""
Sets up the local environment to match my preferences.

pylint:  pylint --max-line-length 120 --no-docstring-rgx '.*'
"""

from __future__ import absolute_import
from __future__ import with_statement

import logging
import os
import os.path
import re
import sys

log = logging.getLogger('scode/rcfiles')

# Subdirectory relative to rcfiles in which to look for dotfiles to install.
DOTFILES_SUBDIR = 'dotfiles'

# Regular expression that filenames must match in order to be picked up (not applied to directories).
# The primary purpose of this is to avoid picking up manually created garbage like editor backup
# files.
DOTFILES_RE = re.compile('^[0-9a-zA-Z-.]*$')

# Enable/disable side-effects to ease debugging during development.
DRY_RUN = False

def ensure_symlink_installed(source_path, dest_path):
    if os.path.islink(dest_path):
        existing_link = os.readlink(dest_path)
        if existing_link == source_path:
            log.debug('OK  %s -> %s', dest_path, source_path)
        else:
            log.info('SM! %s -> %s', dest_path, source_path)
            if not DRY_RUN:
                # XXX(scode): Maybe bother with temp file and atomic rename.
                os.unlink(dest_path)
                os.symlink(source_path, dest_path)
    elif os.path.exists(dest_path):
        log.warn('WRN path exists but is not a symlink, not touching: %s', dest_path)
    else:
        log.info('SM+ %s -> %s', dest_path, source_path)
        if not DRY_RUN:
            os.symlink(source_path, dest_path)

def ensure_directory_exists(path):
    """
    @return True on full success, false if path exists but is not a directory and should be skipped.
    """
    if os.path.exists(path) and not os.path.isdir(path):
        log.warn('WRN path exists but is not a directory (skipping this + all children): %s', path)
        return False

    if os.path.exists(path):
        log.debug('OK  %s', path)
    else:
        log.info('MKD %s', path)
        if not DRY_RUN:
            os.makedirs(path)

    return True

def install_dotfiles(dotfile_dir, user_home):
    """
    Install files starting at rcfiles_home into user_home by creating symbolic links.

    For top-level files in dotfiles_base, the symbolic link name will have a dot prepended
    to it. For files in sub-directories, the files are symlinked with an identical name. For
    example, given this input:

      zshrc
      emacs.d/init.el

    The sumlinks created will be:

      .zshrc -> $dotfiles_base/zshrc
      .emacs.d/init.el -> $dotfiles_base/emacs.d/init.el

    The purpose of this behavior (special-casing the top-level) is to avoid having to
    track files beginning with a dot in the source directory to avoid confusion.
    """
    def dottify(p):
        parent, child = os.path.split(p)
        return os.path.join(parent, '.' + child)
    assert dottify('/home/user/zshrc') == '/home/user/.zshrc'

    def recur(source_dir, target_dir):
        for base_name in os.listdir(source_dir):
            source_path = os.path.join(source_dir, base_name)
            if source_dir == dotfile_dir:
                dest_path = dottify(os.path.join(target_dir, base_name))
            else:
                dest_path = os.path.join(target_dir, base_name)

            if os.path.isdir(source_path):
                if ensure_directory_exists(dest_path):
                    recur(source_path, dest_path)
            else:
                if not DOTFILES_RE.match(base_name):
                    log.warn('WRN skipping because of filename: %s', source_path)
                    continue
                ensure_symlink_installed(source_path, dest_path)

    recur(dotfile_dir, user_home)

def setup_env(rcfiles_home, user_home):
    install_dotfiles(rcfiles_home, user_home)

def main():
    logging.basicConfig(level=logging.INFO)

    rel_rcfiles_home, _ = os.path.split(sys.argv[0])
    rcfiles_home = os.path.abspath(rel_rcfiles_home)
    dotfiles_dir = os.path.join(rcfiles_home, DOTFILES_SUBDIR)
    user_home = os.path.expanduser("~")

    log.debug('rcfiles home: %s', rcfiles_home)
    log.debug('user home:    %s', user_home)

    setup_env(dotfiles_dir, user_home)

if __name__ == '__main__':
    main()
