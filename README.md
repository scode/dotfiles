# dotfiles

A personal dotfiles manager for symlinks and a small amount of managed config state.

## Usage

```bash
# Install every feature whose prerequisites already exist
cargo run -p dotfiles -- install

# Uninstall managed symlinks and removable directories
cargo run -p dotfiles -- uninstall
```

Most install targets are intentionally conditional. Zed files are installed only when `~/.config/zed` already exists.
Claude/Codex dot-directory files are installed only when `~/.claude` or `~/.codex` already exists. The optional
`scode-graphite` skill is installed only when `~/git/scode-graphite-skill` exists, and the optional `scode-voice` skill
is installed only when `~/git/voice` exists. Ghostty config is installed only when `~/Library/Application Support`
exists.

The statusline script is not conditional on Claude or Codex. Install creates `~/bin` when needed and links
`~/bin/claude-statusline.sh`.

NOTE: `uninstall` is not a full rollback for every feature. Claude settings are merged into `~/.claude/settings.json` as
a regular user-owned JSON file, where the installer owns only the specific values it manages. Uninstall removes those
owned values when they still match what install wrote (a value you edited is treated as yours and left alone) and prunes
containers that removal empties, but it never deletes a regular settings file and cannot restore values install
overwrote in the first place. An un-migrated settings symlink left by a very old install is the exception: uninstall
removes that symlink outright, as it did back when the whole file was symlinked. Install may also delete specific values
that older installer versions wrote (retired permissions and hooks); those are one-way cleanups that uninstall does not
re-add. Beyond the empty-container pruning above, values at paths the installer neither manages nor targets for cleanup
are never touched.

## Disclaimer

This is a personal repository shared in case it's useful to others. No backwards compatibility or guarantees about
future behavior are provided.

The lib+bin structure is used to avoid false positive dead code warnings when code is added before it's called from
`main()`. Items exported from `lib.rs` are considered public API by the compiler, suppressing these warnings. However,
there is no actual public API intended for external use.
