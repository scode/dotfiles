# scode/dotfiles

dotfiles (and similar) along with scripting to set up my personal
preferences.

If you use this and you're not me, audit all changes before using
them. No attempt at release engineering/backwards compatibility is
made.

# How to use it
Intended use is to run, from your home directory (with the path to
your checkout appropriately substituted):

  ./git/rcfiles/setup_env.py

It will set up symlinks to configuration files (and in the future,
possibly due addition things, if it isn't already and I just failed to
update the README to match).

# Configuration

The following files affect the behavior of either the setup script or
the scripts it installs:

## ~/.config/scode/dotfiles/kbdtype

Contains the keyboard type. See dotfiles/xkb/*.xkb for available
types.

## ~/.config/scode/dotfiles/profile

Name of host profile (oryxpro, chromebook, ...).

# Other machine bootstrap notes

(not covered by `setup_env.py`)

* [install nix](https://nixos.org/nix/) then follow instructions in `scode-overlay/default.nix`
* ubuntu: either install native i3 to trigger gdm3 autoconf, or [do this](https://askubuntu.com/questions/1103722/how-to-use-custom-xsession-with-gdm3-in-ubuntu-18-04-bionic)
  to convince gdm3. if latter, make ~/.xsession that starts i3; if former, xsessionrc shipped with this repo is fine
* urxvt
* `echo t580 > ./.config/scode/dotfiles/kbdtype`
* `echo t580 > ./.config/scode/dotfiles/profile`
* install pavucontrol. reboot with leaf. start chrome/youtube; pavucontrol now works after pa autostarted on device use.
