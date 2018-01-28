#!/usr/bin/env python3

import abc
import os.path
import re
import sys

PROFILE_PATH = os.path.expanduser('~/.config/scode/dotfiles/profile')


def slurp(path: str) -> str:
    with open(path, 'r') as f:
        return f.read()


def profile() ->  str:
    return slurp(PROFILE_PATH).strip()


def barf(path, s) -> None:
    with open(path, 'w') as f:
        f.write(s)


class IBacklight(abc.ABC):
    def max_brightness(self) -> int:
        raise NotImplemented()

    def current_brightness(self) -> int:
        raise NotImplemented()

    def set_brightness(self, new_brightness: int):
        raise NotImplemented()

class IntelBacklight(IBacklight):
    MAX_BRIGHTNESS_PATH = '/sys/class/backlight/intel_backlight/max_brightness'
    BRIGHTNESS_PATH = '/sys/class/backlight/intel_backlight/brightness'

    def max_brightness(self) -> int:
        return int(slurp(IntelBacklight.MAX_BRIGHTNESS_PATH))

    def current_brightness(self):
        return int(slurp(IntelBacklight.BRIGHTNESS_PATH))

    def set_brightness(self, new_brightness: int):
        barf(IntelBacklight.BRIGHTNESS_PATH, str(new_brightness))

def main(argv):
    backlights = {
        'xps13': IntelBacklight()
    }
    bl = backlights[profile()]

    currb = bl.current_brightness()
    maxb = bl.max_brightness()
    if len(argv) == 1:
        print('current brightness: {} / {}'.format(
            currb,
            maxb,
        ))
    elif re.match('^[0-9]+$', argv[1]):
        print('setting brightnes: {} -> {}'.format(
            currb, argv[1]
        ))
        bl.set_brightness(int(argv[1]))
    else:
        m = re.match('^(\\+|\\-)?([0-9]+)%$', argv[1])
        if m:
            abschange = int(float(m.group(2)) * 0.01 * maxb)
            if m.group(1) == '-':
                newb = max(0, currb - abschange)
            elif m.group(1) == '+':
                newb = min(maxb, currb + abschange)
            else:
                newb = max(0, min(maxb, abschange))
            print('setting brightnes: {} -> {}'.format(
                currb, newb
            ))
            bl.set_brightness(newb)
        else:
            raise Exception('que?')


if __name__ == '__main__':
    main(sys.argv)

