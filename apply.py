#!/usr/bin/env python

from __future__ import absolute_import
from __future__ import with_statement

import logging
import os.path
import sys

log = logging.getLogger('scode/rcfiles')

def main():
    logging.basicConfig(level=logging.INFO)

    rel_rcfiles_home, _ = os.path.split(sys.argv[0])
    rcfiles_home = os.path.abspath(rel_rcfiles_home)
    user_home = os.path.expanduser("~")

    log.info('rcfiles home: %s', rcfiles_home)
    log.info('user home:    %s', user_home)


if __name__ == '__main__':
    main()
