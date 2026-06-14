# SPEC

This file documents repository behavior that is intentional enough that future review should preserve it or update this
file in the same change.

## Invocation Directory

The installer is intended to run from the repository root. Installer source paths are resolved against the process
current working directory, not against the compiled binary path. Those sources are usually under `payload/`, but may
also live elsewhere in the repository when the installed artifact is shared across multiple agent-specific destinations.

This is acceptable for this repository because it is a personal checkout-driven installer, normally run with
`cargo run -p dotfiles -- install` or `cargo run -p dotfiles -- uninstall` from the checkout. Future changes may make
the base directory explicit, but reviews should not treat cwd-relative source lookup as a bug unless the CLI contract
changes at the same time.

## Manual Skill Evals

The `xtask` crate is repository tooling, not an installed dotfiles artifact. `cargo xtask eval ...` is intentionally
manual because it spends model tokens. Eval runs may clone public target repositories under `eval-worktrees/` and write
run artifacts under `eval-runs/`; both directories must stay ignored by git.

The `pre-pr-review-swarm` eval corpus points at unpolished code refs. Historical polished PRs are not ground truth for
these evals. Comparisons are baseline-relative: a likely regression means a judge-approved baseline finding was not
recovered as a judge-approved candidate finding across repeats for the same resolved diff.

## Repo-Owned Symlink Migration

`PayloadSymlink` treats an existing destination symlink to another path inside this repository as old installer state.
On install, it repoints that symlink to the current source path. On uninstall, it removes repo-owned symlinks even when
they point at a previous source path. Symlinks to targets outside the repository must still block install and uninstall
rather than being overwritten or removed.

## Test Coverage Expectations

Installer registration in `src/main.rs` does not need exhaustive integration coverage for every installed source path.
Those registrations are mostly data: paths, feature names, and dependency wiring. A small set of integration tests
should cover the important installer mechanics, but they do not need to duplicate the entire registry.

The library-like pieces of the installer do need good coverage. Feature implementations, graph behavior, path handling,
JSON merging, and migration logic should have direct tests because regressions there can affect many installed files at
once.

## Legacy `old/` Tree

Code under `old/` is legacy reference material. Do not spend review effort on simplification, idiomaticity, style, or
coverage improvements there unless a change explicitly targets that tree.

Security or correctness fixes are still allowed when the user asks for them, but normal repository-wide review should
treat further cleanup in `old/` as out of scope.
