# dist-workspace.toml Template

Use this template as the baseline for a dist release whose Homebrew formula is pulled by `scode/homebrew-dist-tap`.
There is deliberately no installer, no `tap`, and no `publish-jobs`: the release workflow's only job is to build the
four archives and create the GitHub release. The tap fetches those archives by exact name and generates the formula on
its side.

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "REPLACE_WITH_DISCOVERED_DIST_VERSION"
ci = "github"
installers = []
targets = [
  "aarch64-apple-darwin",
  "aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin",
  "x86_64-unknown-linux-gnu",
]
install-path = "CARGO_HOME"
install-updater = true
plan-jobs = ["./release-plan-tests"]
```

## Notes

- Replace `REPLACE_WITH_DISCOVERED_DIST_VERSION` with the version reported by `dist --version` during setup.
- Keep `plan-jobs = ["./release-plan-tests"]` so dist executes the custom release-plan test workflow.
- Keep `targets` exactly as listed, in this order. The tap's updater downloads one `<crate_name>-<target>.tar.xz` per
  target from four fixed URLs: a missing one fails the update, and an extra one is never distributed because nothing
  enumerates release assets. The target set belongs to the tool, not to whoever runs the updater.
- `installers = []` and deleting the key produce byte-identical output (observed on dist 0.31.0 during the treeward
  migration and again on 0.32.0). The explicit empty list is kept so a reader sees the decision rather than an omission.
- There is no comment explaining the missing installer, on purpose: `dist init` rewrites this file in its own layout and
  drops custom comments, so on a new repository such a comment would not survive setup. `dist generate` leaves comments
  alone, which is why saltybox's file carries one; adding one after `dist init` is fine, but keep the Verification
  Checklist's key-based negative check in mind rather than a word match.
- Regenerate `release.yml` after any change here with the dist version pinned by `cargo-dist-version` (`dist generate`,
  then `dist generate --check`), never with `dist init`, which would bump the pin. See Phase B and E in `SKILL.md`.
