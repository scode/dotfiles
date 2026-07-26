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
mismatched execution modes, mismatched efforts within one backend, and cross-backend comparisons where either run left
`--effort` unset (an unset effort is each vendor's own built-in default, and defaults are not comparable across
backends). Legacy restricted artifacts count as direct-reviewer runs because that execution path did not change; legacy
unrestricted artifacts used harness-owned panel fan-out and are not native-swarm baselines.

A native-swarm repeat is valid only when the coordinator's reviewer accounting agrees with successful collaboration
calls in the parent transcript. Failed spawns and continuation messages do not count. The run must abort instead of
accepting coordinator-only review, a partial panel, condition-excluded reviewers reported as completed, or findings with
duplicate identifiers. These refusals are intentional guards, not over-strictness to relax in review.

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

## Managed Text Block Ownership

`ManagedBlock` owns a marker-delimited region of a user-owned text file. The owned region is the BEGIN marker line
through the END marker line, inclusive; everything outside it belongs to the user or to another tool and is never
reordered, reindented, or rewritten. Install writes the body unconditionally, so edits inside the fence are lost with no
journal to restore them. Uninstall removes the region whether or not the body still matches what install would write:
the markers are the ownership claim, not the bytes between them. That is a deliberate divergence from `JsonManaged`,
where an edited value is treated as reclaimed by the user — a managed JSON path shares a namespace with the user's own
keys, while a fenced region does not. The destination file is never deleted, even when the block was all it contained.

Insertion has two carve-outs from "never touches anything outside the region", and both are one-way. A blank line
separates the block from neighboring content, and a final line that lacked a newline gets one, since otherwise the BEGIN
marker would be welded onto the end of the user's last statement. Uninstall reclaims neither, because it owns only what
is between the markers. A later install reuses an existing blank separator instead of adding another, so repeated
install/uninstall cycles converge instead of growing the file.

Those carve-outs are about the file's text. The file's metadata is a separate matter: every write replaces the
destination with a new inode, so hard links break and owner, group, extended attributes, and ACLs are not carried onto
the replacement. See the Atomic File Replacement section below.

Position is decided on first insertion and preserved after that. Install updates a block where it already sits rather
than moving it, because relocating it would reorder the content against whatever another tool appended in the meantime.

A missing destination is a policy choice, not a fixed behavior: `MissingDestination::Skip` (the default) treats absence
as "this does not apply on this machine" and installs nothing, while `Create` writes a new file containing only the
block. `Create` never creates parent directories — that is `ManagedDirectory`'s job — so a destination under a missing
directory fails.

`Skip` is a statement about the destination, not a promise that the registration goes unchecked. The payload body is
read and validated before the destination is examined, so a payload that is missing, not valid UTF-8, or carrying a
marker line of its own fails install even where the block does not apply. That ordering is deliberate: those are faults
in the repository, identical on every machine running the same commit, and a fault that deterministic deserves a
deterministic report rather than one that appears only on whichever machines happen to have the destination file.

Marker matching keys only on the `<comment-prefix> BEGIN managed-block(scode-dotfiles/<id>)` prefix, with leading
whitespace tolerated on both the line and the configured prefix. The human-readable notice after the key is not matched
and may be reworded; the key itself may not, because it is the only thing that identifies a block already installed on a
machine.

Changing a block's id, comment prefix, or destination is therefore a migration rather than an edit. All three behave the
same way: the new block is installed everywhere, and the old one is orphaned on every machine that already installed,
because nothing recognizes it any more. Each needs a `DeleteManagedBlock` naming the old destination and the old marker,
which is install-only cleanup and is never inverted on uninstall.

Changing a block's **position** is a different problem with no migration path. The marker does not change, so install
finds the block where it already sits and leaves it there: the new position takes effect only on machines that have
never installed. A `DeleteManagedBlock` naming that marker does not fix it. Ordering the delete before the install is
declarable — that is what `depends_on` is for — but the pair never converges, because the marker the cleanup names is
the one the install writes back. Every subsequent run deletes the block and re-inserts it, reporting `Changed` and
rewriting the user's file forever, and a failure between the two steps leaves the machine with no block at all. Moving
an installed block therefore means renaming its id (with a `DeleteManagedBlock` for the old one), or accepting the
placement it already has.

Two properties of the key are load-bearing rather than incidental. It ends at the closing parenthesis, which is the only
reason id `bash` cannot prefix-match `bash-extra` and rewrite its neighbor's region; and the comment prefix is trimmed
at construction — and rejected outright when nothing remains — without which an empty or whitespace-led prefix would
produce a key starting with a space that can never match the marker it just wrote. Note that indentation is tolerated
when matching but not preserved when rewriting: an indented block is found and then re-emitted at column zero, so a
destination where indentation carries meaning is out of scope for this feature as it stands.

`ManagedBlock` is not safe against a concurrent writer: the destination is read, edited in memory, and renamed back, so
a write landing in that window is lost silently. This is accepted, not unsolved. Advisory locking only works when every
writer participates, and the tools this feature exists to coexist with do not lock the files in question.

Ambiguous marker states fail rather than get repaired: a BEGIN with no END, an END before its BEGIN, or two blocks
sharing an id. Repairing a half-present block by appending a fresh one would duplicate its content on every subsequent
install. A symlinked destination also fails, matching how the symlink features treat unexpected symlinks, as does a
destination that is not a regular file or not valid UTF-8 — the latter deliberately, since reading it lossily would let
the next install atomically overwrite the user's file with replacement characters.

A block body containing a marker line is rejected at install time, before anything is written: writing it would produce
exactly the duplicate-marker state above, and from then on both install and uninstall would refuse to act, leaving
hand-editing the destination as the only way out. The rejection is deliberately wider than the block being installed —
it matches any `scode-dotfiles` marker regardless of id or comment prefix, anywhere in the line — because a neighboring
block sharing the destination is wedged just as permanently, and a payload seeded from a real file is at least as likely
to carry someone else's marker as its own. The price is that a payload cannot contain the marker text at all, even in
prose.

Block bodies are copied from payload files rather than sourced from them at runtime. A `source`-shim would keep the
destination stable and propagate repository changes without an install, but it hides the content from the user and from
other tools reading the file. For files as unforgiving as shell startup files, content that changes only when the
installer runs is the intended trade.

## Atomic File Replacement

Every feature that rewrites a user-owned file — `ManagedBlock` and `JsonManaged` — does so via `write_file_atomically`:
a temporary file in the destination's own directory, fsynced, renamed over the destination, followed by an fsync of the
directory. A reader therefore sees either the entire old file or the entire new one. This matters because these files
are read by other tools on their own schedule, so a truncated write surfaces later and elsewhere — at the next login, or
as malformed JSON in an unrelated program — rather than during install. New features that rewrite user-owned files
should use the same helper rather than `fs::write`.

Permissions follow one rule: the installer never widens. An existing regular file keeps its own permission bits, wide
ones included — preserving what the user set is not the same as choosing it. A file created from scratch requests 0644
and lets the umask narrow that, which is why the temporary file is opened by hand rather than through `NamedTempFile`,
whose 0600 would otherwise leak into a new destination. A user running `umask 077` means it.

0644 rather than the 0666 `File::create` requests, deliberately: matching `File::create` would leave the decision
entirely to the umask, and a permissive one (`umask 002` is a distribution default, `umask 000` appears in container
images) would then have the installer author a group- or world-_writable_ shell startup file. Nothing downstream rejects
one the way sshd rejects a writable `~/.ssh`. Narrowing belongs to the user; widening is not on offer.

The mode reaches `open(2)` rather than a chmod afterwards, and that is security-relevant rather than incidental: a
temporary file created at the umask default while replacing a 0600 destination would exist, however briefly, wider than
the file it replaces, and a local racer who opens it in that window keeps a readable descriptor across any later
narrowing. The open-time mode is the whole guarantee. The follow-up chmod only ever widens — it restores bits an
over-strict umask cleared — so its position relative to the write is not a security property in either direction, and
review should not defend it as one.

setuid, setgid, and the sticky bit are deliberately not carried across: the replacement is a different inode owned by
the installing user and group, so preserving them would re-point them at a different principal. Owner, group, extended
attributes, and ACLs are lost too, but as an inherent consequence of replace-by-rename rather than a decision — the
replacement is a new inode and carries only what is copied onto it explicitly. Replacing a file also requires write
permission on its directory rather than on the file itself.

ACL loss has one sharp edge worth naming: on a file carrying a POSIX ACL, the group bits of `st_mode` are the ACL mask,
not the group entry. Preserving those bits onto the replacement turns the mask into real group permissions, so the new
file can grant the owning group more than the ACL did — a narrow exception to "the installer never widens". This is
accepted rather than defended against: destinations here are personal dotfiles, and detecting ACLs to fail or fall back
to an in-place write would buy little for the checkout-driven single-user setup this installer targets.

Flushing the directory entry after the rename is best-effort, and only in one specific sense: when the filesystem
reports that it does not implement the operation (`ENOTSUP`, `EOPNOTSUPP`, `EINVAL`, `ENOTTY`), the write is already in
place via `rename(2)` and the install succeeds. That is a static property of the mount rather than a lost write, and
failing on it would make every install on a network-mounted home directory report failure forever. The strong barrier is
tried first — on Apple targets `File::sync_all` is `F_FULLFSYNC`, which is documented only for HFS+, FAT, UDF and APFS —
then a plain `fsync(2)`. Any other error, `EIO` above all, still fails the feature: that one means the flush genuinely
did not happen.

A feature reporting failure does not mean it changed nothing. Replacement is atomic but it is not the last step:
flushing the directory entry happens after the rename, so a failure there leaves the new contents in place and reports a
durability problem, not an unwritten file. Errors are worded to say which of the two happened, and neither the installer
nor anything reading its output should treat "failed" as "rolled back" — nothing here rolls back.

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
