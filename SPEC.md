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

`compare` deliberately refuses runs whose configurations measure different things: mismatched reviewer restrictions,
mismatched efforts within one backend, and cross-backend comparisons where either run left `--effort` unset (an unset
effort is each vendor's own built-in default, and defaults are not comparable across backends). These refusals are
intentional guards, not over-strictness to relax in review.

## Repo-Owned Symlink Migration

`PayloadSymlink` treats an existing destination symlink to another path inside this repository as old installer state.
On install, it repoints that symlink to the current source path. On uninstall, it removes repo-owned symlinks even when
they point at a previous source path. Symlinks to targets outside the repository must still block install and uninstall
rather than being overwritten or removed.

## Managed JSON Ownership

`JsonManaged` edits user-owned JSON files and owns only the paths its `managed_*` operations declare. Declaring a path
managed is an ownership claim scoped to the owned value itself: `managed_value` writes its value unconditionally,
clobbering pre-existing user state at that path with no pre-install journal to restore it from, while
`managed_strings_in_array` merges — it adds missing owned strings, preserves neighboring user-added strings, and fails
on shape violations rather than overwriting the container. Uninstall removes the owned footprint — a managed value is
deleted when it still matches what install would write, and containers the removal empties are pruned. A managed value
the user has edited is treated as reclaimed and survives uninstall; type changes along a managed path (a scalar where
install would create an object, a non-string entry in a managed string array) are user edits too, so uninstall treats
them as reclaimed rather than failing. Reclaimed-not-failing is scoped to edits along managed paths: whole-file problems
(malformed JSON, a non-object root, an unrecognized symlink) fail uninstall the same way they fail install. A regular
file is never deleted, and apart from pruning ancestor containers that a managed removal leaves empty, values not named
by any operation are never touched.

The condition-gated variant (`managed_value_if_path_exists`) keys install on a filesystem path: present means enforce
the value, absent means remove it, so the config never advertises an artifact that is not installed. Uninstall ignores
the condition and removes the still-matching value either way.

An un-migrated legacy settings symlink follows the same rule as the Repo-Owned Symlink Migration section above: install
migrates it to a regular file, and uninstall removes it outright rather than leaving a repo-owned link dangling at a
deleted payload path.

The `remove_*` operations are deliberately one-way: they retire values older installer versions wrote, run on install
only, and must never be inverted on uninstall. Reviews should preserve this split — an operation that "ensures" state
without a revert story belongs in the cleanup family or needs an explicit new contract here.

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
