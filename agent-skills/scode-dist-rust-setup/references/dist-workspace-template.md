# dist-workspace.toml Template

Use this template as the baseline for dist + Homebrew publishing.

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "REPLACE_WITH_DISCOVERED_DIST_VERSION"
ci = "github"
installers = ["homebrew"]
targets = [
  "aarch64-apple-darwin",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
]
install-path = "CARGO_HOME"
install-updater = true
tap = "scode/homebrew-dist-tap"
publish-jobs = ["homebrew"]
plan-jobs = ["./release-plan-tests"]
```

## Notes

- Replace `REPLACE_WITH_DISCOVERED_DIST_VERSION` with the version reported by `dist --version` during setup.
- Keep `plan-jobs = ["./release-plan-tests"]` so dist executes the custom release-plan test workflow.
- Re-run `dist init` after any `dist-workspace.toml` changes.
