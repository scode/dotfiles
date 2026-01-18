# dotfiles

A personal dotfiles manager that installs and uninstalls personal configuration.

## Usage

```bash
# Install all features
cargo run -- install

# Uninstall all features
cargo run -- uninstall
```

## Disclaimer

This is a personal repository shared in case it's useful to others. No backwards compatibility or guarantees about
future behavior are provided.

The lib+bin structure is used to avoid false positive dead code warnings when code is added before it's called from
`main()`. Items exported from `lib.rs` are considered public API by the compiler, suppressing these warnings. However,
there is no actual public API intended for external use.
