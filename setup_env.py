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

DOTFILES_SUBDIR = 'dotfiles'
FILES_SUBDIR = 'files'

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

def install_symlinks(src_dir, dst_dir, prefix=''):
    for src_name in os.listdir(src_dir):
        src_path = os.path.join(src_dir, src_name)
        dst_path = os.path.join(dst_dir, prefix + src_name)

        ensure_symlink_installed(src_path,dst_path)

def main():
    logging.basicConfig(level=logging.INFO)

    rel_rcfiles_home, _ = os.path.split(sys.argv[0])
    rcfiles_home = os.path.abspath(rel_rcfiles_home)
    user_home = os.path.expanduser("~")

    log.debug('rcfiles home: %s', rcfiles_home)
    log.debug('user home:    %s', user_home)

    install_symlinks(os.path.join(rcfiles_home, FILES_SUBDIR), user_home)
    install_symlinks(os.path.join(rcfiles_home, DOTFILES_SUBDIR), user_home, prefix='.')

if __name__ == '__main__':
    main()
