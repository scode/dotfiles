# Dotfiles Installer

## Migrating features in main.rs

Previous versions of this tool have been used to install symlinks, directories, etc. on
real systems. When modifying `src/main.rs`, always account for the old installation state
that may exist on machines that ran a previous version.

Examples:

- **Moving a symlink destination** (PayloadSymlink or RawSymlink destination changes from
  Y to X): add a `DeleteSymlink::new("Y")` to clean up the old path.
- **Removing a symlink feature entirely**: replace it with a `DeleteSymlink::new(...)` for
  the old destination so it gets cleaned up.
- **Moving files between directories**: both add the new symlink at the new path *and*
  add a `DeleteSymlink` for the old path. See the `delete-claude-agent-*` entries in
  `add_claude_features` for a real example of this pattern.

The general rule: never assume a clean slate. If a path was previously installed, emit a
removal feature for it.

## Before finishing work

The following must all pass before creating a PR or claiming work is done:

- `dprint fmt`
- `cargo test`
- `cargo clippy`
