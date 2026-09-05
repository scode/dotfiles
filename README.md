# dotfiles

A personal dotfiles manager for symlinks, a small amount of managed config state, and marker-fenced blocks inside files
that belong to the user.

## Usage

```bash
# Install every feature whose prerequisites already exist
cargo run -p dotfiles -- install

# Uninstall managed symlinks and removable directories
cargo run -p dotfiles -- uninstall
```

Most install targets are intentionally conditional. Zed files are installed only when `~/.config/zed` already exists.
Claude/Codex dot-directory files are installed only when `~/.claude` or `~/.codex` already exists. Agent skills are also
installed for Muse Code and OpenCode, into `~/.config/muse/skills` and `~/.config/opencode/skills`, each only when the
harness's config directory (`~/.config/muse`, `~/.config/opencode`) already exists. The optional `scode-graphite` skill
is installed only when `~/git/scode-graphite-skill` exists, and the optional `scode-voice` skill is installed only when
`~/git/voice` exists. Ghostty config is installed only when `~/Library/Application Support` exists.

The statusline script is not conditional on Claude or Codex. Install creates `~/bin` when needed and links
`~/bin/claude-statusline.sh`.

`~/.bashrc` and `~/.zshrc` are also unconditional, and they are the targets where install edits a plain-text file it
does not own (the other non-owned file is `~/.claude/settings.json`, described below). Both receive the same block of
shell aliases from `payload/shellrc`. The installer claims a region delimited by
`# BEGIN managed-block(scode-dotfiles/bash)` (or `.../zsh`) and a matching `END` line — other tools and your own edits
can append, prepend, and rearrange freely around it. The only things install writes outside the markers are a blank line
separating the block from its neighbors and, if your file did not end in a newline, that newline. Anything you write
_between_ the markers is overwritten on the next install. Either file is created if it does not exist yet, so a machine
that never runs zsh ends up with a small `~/.zshrc` it does not use. The zsh path is always `~/.zshrc`; a setup that
relocates startup files with `ZDOTDIR` gets a block zsh never reads, and needs to source it from the real one. If your
`~/.bashrc` or `~/.zshrc` is a symlink — a common setup when it is managed from another checkout — install refuses it
rather than writing through the link, and reports that feature as failed.

NOTE: `uninstall` is not a full rollback for every feature. Claude settings are merged into `~/.claude/settings.json` as
a regular user-owned JSON file, where the installer owns only the specific values it manages. Uninstall removes those
owned values when they still match what install wrote (a value you edited is treated as yours and left alone) and prunes
containers that removal empties, but it never deletes a regular settings file and cannot restore values install
overwrote in the first place. An un-migrated settings symlink left by a very old install is the exception: uninstall
removes that symlink outright, as it did back when the whole file was symlinked. Install may also delete specific values
that older installer versions wrote (retired permissions and hooks); those are one-way cleanups that uninstall does not
re-add. Beyond the empty-container pruning above, values at paths the installer neither manages nor targets for cleanup
are never touched.

The `~/.bashrc` and `~/.zshrc` blocks are the same story in a different file: uninstall removes the marked region but
never the file, not even when the block was all it contained. The blank line install inserted to separate the block from
its neighbors stays behind, since it sits outside the markers.

## Disclaimer

This is a personal repository shared in case it's useful to others. No backwards compatibility or guarantees about
future behavior are provided.

The lib+bin structure is used to avoid false positive dead code warnings when code is added before it's called from
`main()`. Items exported from `lib.rs` are considered public API by the compiler, suppressing these warnings. However,
there is no actual public API intended for external use.
