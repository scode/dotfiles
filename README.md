# dotfiles

A personal dotfiles manager for symlinks and a small amount of managed config state.

## Usage

```bash
# Install every feature whose prerequisites already exist
cargo run -- install

# Uninstall managed symlinks and removable directories
cargo run -- uninstall
```

Install is intentionally conditional. Zed files are installed only when `~/.config/zed` already exists. Claude/Codex
dot-directory files are installed only when `~/.claude` or `~/.codex` already exists. The optional `scode-graphite`
skill is installed only when `~/git/scode-graphite-skill` exists, and the optional `scode-voice` skill is installed only
when `~/git/voice` exists. Ghostty config is installed only when `~/Library/Application Support` exists.

NOTE: `uninstall` is not a full rollback for every feature. In particular, Claude settings are now merged into
`~/.claude/settings.json` as a regular user-owned JSON file. The installer stops managing that file on uninstall, but it
does not remove or revert it.

## Claude Code Plugins

This repo is also a Claude Code plugin marketplace containing:

- **codex-code-review** - Code review using OpenAI's Codex CLI
- **gemini-code-review** - Code review using Google's Gemini CLI

NOTE: This is still just a personal thing. Those are not meant for wide use and may contain assumptions or preferences
that are intentionally personal and non-configurable.

### Installation (remote)

```bash
/plugin marketplace add scode/dotfiles
/plugin install codex-code-review@dotfiles
/plugin install gemini-code-review@dotfiles
```

### Installation (local development)

```bash
/plugin marketplace add /path/to/dotfiles
/plugin install codex-code-review@dotfiles
/plugin install gemini-code-review@dotfiles
```

## Disclaimer

This is a personal repository shared in case it's useful to others. No backwards compatibility or guarantees about
future behavior are provided.

The lib+bin structure is used to avoid false positive dead code warnings when code is added before it's called from
`main()`. Items exported from `lib.rs` are considered public API by the compiler, suppressing these warnings. However,
there is no actual public API intended for external use.
