# Dotfiles Installer

## Keeping main.rs in sync

This repo is a dotfiles installer. Files under `payload/`, `agent-instructions/`, and `agent-skills/` are installed to
the home directory via symlinks registered in `src/main.rs`. When you add, remove, rename, or move an installed source
file, you must update `src/main.rs` to match — otherwise the file won't actually be installed. Look at existing entries
in `main.rs` for the pattern.

## Migrating features in main.rs

Previous versions of this tool have been used to install symlinks, directories, etc. on real systems. When modifying
`src/main.rs`, always account for the old installation state that may exist on machines that ran a previous version.

Examples:

- **Moving a symlink destination** (PayloadSymlink or RawSymlink destination changes from Y to X): add a
  `DeleteSymlink::new("Y")` to clean up the old path.
- **Removing a symlink feature entirely**: replace it with a `DeleteSymlink::new(...)` for the old destination so it
  gets cleaned up.
- **Moving installed destinations between directories**: both add the new symlink at the new destination _and_ add a
  `DeleteSymlink` for the old destination. See the `delete-claude-agent-*` entries in `add_claude_features` for a real
  example of this pattern.
- **Moving only the repository source path**: keep the installed destination the same and update the `PayloadSymlink`
  source. The installer will repoint old repo-owned symlinks during the next install.
- **Replacing a symlinked config blob with in-place managed state**: teach the new feature how to recognize and migrate
  the old installer-owned symlink. Do not assume every former symlink should be deleted outright; some migrations, like
  `~/.claude/settings.json`, need to replace the symlink with a regular user-owned file and then manage only a subset of
  its contents.

The general rule: never assume a clean slate. If a path was previously installed, emit a removal feature for it.

## Agent Skills

Shared agent skills live in `agent-skills/`. When the user asks to view, modify, or discuss a skill, look there — not in
`~/.claude/skills/`, `~/.codex/skills/`, or the old `payload/dot_claude/skills/` path.

## Agent Instructions

Shared installed agent instructions live in `agent-instructions/`. When changing the Claude/Codex instruction blob that
gets installed as `~/.claude/CLAUDE.md` and `~/.codex/AGENTS.md`, edit `agent-instructions/AGENTS.md`, not the old
`payload/dot_claude/CLAUDE.md` path.

## Lore

`lore/*.md` notes, excluding local agent instructions, are historical artifacts. Do not eagerly read them during normal
work; use them only when digging into past motivation, decisions, or context.

## SPEC.md

Agents must conform to `SPEC.md`. If implementation and `SPEC.md` disagree, treat that as a bug or explicitly update
`SPEC.md` in the same change.

## Choosing `feat` vs `docs` for commit types

This repo is an installer — its product is the set of files it installs. Changes to installed artifacts (configs,
skills, shell scripts, etc.) are changes to what gets installed, even when those files happen to be markdown or prose.
Use `feat` or `fix` for those, not `docs`.

`docs` is for documentation _about this project itself_: the README, CLAUDE.md, code comments in `src/`, and similar.

Skills (`agent-skills/`) are a common source of confusion because they look like documentation, but they are installed
artifacts — part of the product. Adding, removing, or changing a skill's behavior is `feat` (or `fix`).

## Commit messages and PR titles

All commit messages and PR titles must use Conventional Commit format: `<type>: <short summary>`

Allowed types: `feat`, `fix`, `docs`, `perf`, `refactor`, `style`, `test`, `chore`, `ci`, `revert`.

Append `!` after the type for breaking changes (e.g. `feat!: remove legacy endpoint`). Scope is optional.

Rules:

- The title describes _what_ the change does, focusing on the user-visible effect rather than implementation details. A
  bug fix that requires heavy refactoring is `fix`, not `refactor`. A new CLI flag is `feat`, not `chore`.
- The summary after the colon is lowercase, imperative mood, no trailing period.
- Keep the first line under 72 characters.

## Commit bodies and PR descriptions

The body/description covers the _why_ — context that cannot be inferred from the code or the title. Leave it empty only
when the title and diff really do say everything useful, such as a trivial one-line fix with no migration concern,
surprising constraint, or follow-up risk.

Write a body when there is non-obvious context a future reader would otherwise have to rediscover: why the change is
needed, what old state or upgrade path it preserves, why a boundary matters, what tradeoff was chosen, what is
intentionally omitted, or how a stacked PR depends on it. For large changes, a brief summary of _what_ is acceptable to
aid skimming, but the primary purpose is still the _why_.

Do not include "Generated with Claude Code" badges or similar attribution lines.

## Writing voice

Use the `scode-voice` skill when writing or editing any prose destined for files: documentation, READMEs, doc comments,
inline comments, commit messages, and PR descriptions. If the project's own CLAUDE.md specifies a different voice or
writing style, that takes precedence.

## Before finishing work

The following must all pass before creating a PR or claiming work is done:

- `dprint fmt`
- `cargo fmt`
- `cargo test`
- `cargo clippy`
