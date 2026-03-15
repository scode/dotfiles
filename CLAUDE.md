# Dotfiles Installer

## Keeping main.rs in sync

This repo is a dotfiles installer. Files under `payload/` are installed to the home directory via symlinks registered in
`src/main.rs`. When you add, remove, rename, or move a payload file, you must update `src/main.rs` to match — otherwise
the file won't actually be installed. Look at existing entries in `main.rs` for the pattern.

## Migrating features in main.rs

Previous versions of this tool have been used to install symlinks, directories, etc. on real systems. When modifying
`src/main.rs`, always account for the old installation state that may exist on machines that ran a previous version.

Examples:

- **Moving a symlink destination** (PayloadSymlink or RawSymlink destination changes from Y to X): add a
  `DeleteSymlink::new("Y")` to clean up the old path.
- **Removing a symlink feature entirely**: replace it with a `DeleteSymlink::new(...)` for the old destination so it
  gets cleaned up.
- **Moving files between directories**: both add the new symlink at the new path _and_ add a `DeleteSymlink` for the old
  path. See the `delete-claude-agent-*` entries in `add_claude_features` for a real example of this pattern.

The general rule: never assume a clean slate. If a path was previously installed, emit a removal feature for it.

## Skills

Claude Code skills live in `payload/dot_claude/skills/`. When the user asks to view, modify, or discuss a skill, look
there — not in `~/.claude/skills/` or elsewhere.

## Before finishing work

The following must all pass before creating a PR or claiming work is done:

- `dprint fmt`
- `cargo test`
- `cargo clippy`
