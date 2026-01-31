# dotfiles

A personal dotfiles manager that installs and uninstalls personal configuration.

## Usage

```bash
# Install all features
cargo run -- install

# Uninstall all features
cargo run -- uninstall
```

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
