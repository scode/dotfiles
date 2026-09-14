---
name: scode-dist-rust-setup
description: Set up or standardize a Rust repository with cargo-dist release automation, Linux-focused CI with macOS release-plan tag gates, git-cliff changelog generation, Conventional Commit PR title enforcement, and release archives shaped for scode/homebrew-dist-tap to pull; the repository never pushes a Homebrew formula. Use when creating a new Rust release pipeline or migrating an existing repo to this exact distribution model.
---

# Scode Dist Rust Setup

Set up a Rust repository to match the release/distribution pattern used in treeward and saltybox: dist-generated release
workflow that builds four archives and creates the GitHub release, Linux-focused CI, macOS release-plan gating on tags,
and git-cliff changelog governance. Homebrew distribution is pull-based: `scode/homebrew-dist-tap` fetches a chosen
release tag, validates the archives, records their hashes, and generates the formula there, behind human review. Nothing
this skill installs pushes a formula, holds a tap credential, or generates a `.rb` file.

NOTE: This skill does not touch the tap. Onboarding a tool in `scode/homebrew-dist-tap` (its per-tool updater, its
`pull.toml` entry, its formula test) is the tap owner's work, done in the tap repository. What this skill owns is the
repository side: producing releases that satisfy the tap's archive contract, and documenting the hand-off.

## Why pull, not push

Push publishing meant every tool's release workflow held a write token for the tap, and tagging a release silently
rewrote a formula other people install from. The tap owner wants formula publication to be an explicit, reviewed
decision made in the tap, and once every tool pulls, the tap's write credentials go away. treeward (PR #141) and
saltybox (PR #252) have been migrated; this skill installs what those repositories now have. Do not add a `tap`,
`publish-jobs`, or a `homebrew` installer even if a user's older repository still has them; if asked to set up a
repository that currently pushes, the setup is the migration.

### Migration scope

A repository that already pushes was set up by an earlier version of this skill, so most of what the phases below
install is already there. The migration is the small diff treeward and saltybox landed, three files each:
`dist-workspace.toml`, the regenerated `release.yml`, and the Releasing and preflight prose in `CONTRIBUTING.md`. On
such a repository run Phase A (its discovery feeds every later phase), Phase B, Phase C, Phase E, the CONTRIBUTING part
of Phase I (step 4), and Phase J, and leave `ci.yml`, `cliff.toml`, the PR-title workflow, `CLAUDE.md`, and `README.md`
alone unless the user asks for full standardization. In `CONTRIBUTING.md`, change only what the push model touched: the
release watch step and the hand-off step at the end of Releasing, and the preflight items about `tap`, `publish-jobs`,
secrets, and the archive contract. Keep every repository-local preflight item that is already there; saltybox's rules
about `allow-dirty` and its permission hardening are exactly the documentation Phase E later relies on. Title the PR
`ci: stop publishing the homebrew formula from the release workflow` with `changelog: skip`, and make it a draft: the
owner sequences the merge with enabling the tool's pull entry in the tap.

One saltybox-local choice is deliberately not installed: saltybox hand-patches the generated `release.yml` so that only
the tag-gated `host` job gets `contents: write`, with `allow-dirty = ["ci"]` to let dist tolerate the edit. This skill
keeps `release.yml` dist-managed and unedited. Adopting that hardening is a per-repository decision, not part of this
setup. It does change how regeneration works on such a repository; see the `allow-dirty` case in Phase E.

## Required Inputs

Collect these values before making changes:

- `crate_name`: Required. Read from `Cargo.toml` (`[package].name`).
- `github_owner/repo`: Derive from `git remote get-url origin`. Prompt only if parsing is ambiguous.
- `cargo_dist_version`: on a new repository, the version of the dist you install; on a repository with an existing
  `cargo-dist-version`, that pin, unchanged. Phase B says how to get a matching binary.

## Hard Defaults

Apply these defaults unless the user explicitly asks to diverge:

- Homebrew tap repository, for the README install line only: `scode/homebrew-dist-tap`, installed as
  `brew install scode/dist-tap/<crate_name>`
- Dist installers: none (`installers = []`). No `tap`, no `publish-jobs`, no tap token secret.
- Dist targets, in this order to match the template and the migrated repositories: `aarch64-apple-darwin`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`
- Dist plan hook: `plan-jobs = ["./release-plan-tests"]`
- CI platform focus: Linux for standard CI, macOS only as tag-gated release-plan test
- Release secrets: `GITHUB_TOKEN` and nothing else

### The archive contract the tap depends on

The tap's updater consumes each release by a fixed contract with no fallback and no auto-discovery: it fetches exactly
four URLs and validates what comes back. A missing archive fails the update; an extra target is not rejected, it is
simply never distributed, because nothing enumerates release assets. The defaults above exist to satisfy the contract,
and changing any of this in a tool's repository needs a coordinated change to the tap's per-tool code first. Treat it as
frozen:

| Item             | Required by the tap                                                                                            |
| ---------------- | -------------------------------------------------------------------------------------------------------------- |
| Tag format       | `v`-prefixed SemVer, e.g. `v0.3.3`                                                                             |
| Targets          | exactly the four listed above                                                                                  |
| Asset URL        | `https://github.com/scode/<crate_name>/releases/download/<tag>/<crate_name>-<target>.tar.xz`                   |
| Archive format   | one xz stream containing one tar (dist's default for these targets)                                            |
| Archive contents | exactly `<crate_name>-<target>/` holding `README.md`, `CHANGELOG.md`, `LICENSE`, and the `<crate_name>` binary |

The `<crate_name>-<target>-update` files that `dist plan` lists come from `install-updater = true`; they are separate
release assets, not archive members, and the tap ignores them. The tap records the SHA-256 of each archive it accepts
and treats different bytes at the same tag as an integrity error, so assets on a published tag are never re-uploaded or
replaced.

## Workflow

### Phase A: Discover Project Facts

1. Confirm the repository root contains `Cargo.toml`.
2. Extract `crate_name` from `Cargo.toml`.
3. Derive `github_owner/repo` from `git remote get-url origin`.
4. Check that `github_owner/repo` is `scode/<crate_name>`. The tap's asset URL contract hardcodes that shape, so a
   repository named differently from its crate, or a crate whose binary is not named after the package (a `[[bin]]` with
   its own `name`), cannot be onboarded without tap-side work. Flag it to the user before going further.
5. Detect existing files that may need updates instead of replacement, and note whether `dist-workspace.toml` has an
   `allow-dirty` entry containing `"ci"` (it changes how Phase E works):
   - `dist-workspace.toml`
   - `.github/workflows/ci.yml`
   - `.github/workflows/release.yml`
   - `.github/workflows/release-plan-tests.yml`
   - `.github/workflows/conventional-commit-pr-title.yml` (PR title + changelog decision validation)
   - `cliff.toml`
   - `CONTRIBUTING.md`
   - `README.md`

### Phase B: Pick the dist Version and Get a Matching Binary

Which dist runs matters, because dist refuses to work across a version mismatch and `dist init` resolves a mismatch by
rewriting the pin. Two cases:

- **New repository, no `cargo-dist-version` yet.** Install or update dist, capture `dist --version`, and pin that exact
  string as `cargo-dist-version` in `dist-workspace.toml`. Never leave it unpinned.
- **Existing pin.** Do not change the pin. If the installed dist is not exactly the pinned version, fetch the pinned
  release binary into a scratch directory outside the repository and use it for every dist command in this setup
  (substitute the host's target triple; this is the Linux x86_64 form):

  ```sh
  V=<cargo-dist-version from dist-workspace.toml>
  cd "$(mktemp -d)" || exit 1
  curl -sSfL -o d.tar.xz "https://github.com/axodotdev/cargo-dist/releases/download/v$V/cargo-dist-x86_64-unknown-linux-gnu.tar.xz"
  tar xf d.tar.xz
  D=$PWD/cargo-dist-x86_64-unknown-linux-gnu/dist
  $D --version    # must print $V
  cd - >/dev/null
  ```

  Why: a dist whose version differs from the pin refuses `generate`, `generate --check`, and `plan` outright with a
  mismatch error, and `dist init` instead "fixes" the mismatch by rewriting `cargo-dist-version` to its own version and
  regenerating `release.yml`, silently and without a prompt under `--yes`. An agent running `dist init` with whatever is
  on `PATH` therefore opens a PR that bumps dist as a side effect, which neither treeward's nor saltybox's migration
  did; bumping the pin is its own deliberate change. The `cd` in the snippet matters too: run in the repository root,
  the download and the unpacked directory become untracked files that the sweep in Phase J and the clean-tree check at
  release time trip over.

  Later phases write `$D` for "the dist binary matching the pin". On a new repository that is just the installed `dist`.

### Phase C: Configure dist-workspace.toml

1. Create or update `dist-workspace.toml` using `references/dist-workspace-template.md`.
2. Keep these values exact unless the user explicitly asks otherwise:
   - `ci = "github"`
   - `installers = []`
   - `targets` exactly the four in Hard Defaults
   - `install-path = "CARGO_HOME"`
   - `install-updater = true`
   - `plan-jobs = ["./release-plan-tests"]`
3. Make sure there is no `tap` key, no `publish-jobs` key, and no `homebrew` in `installers`. Check `Cargo.toml` too:
   dist also reads `[package.metadata.dist]`, and a `homebrew` installer or `tap` there attaches a `<crate_name>.rb` to
   every release even when `dist-workspace.toml` is clean; with `publish-jobs` there as well, the publish job comes back
   too. (`[workspace.metadata.dist]` alongside `dist-workspace.toml` is a hard error, so it cannot survive unnoticed.)
   On a repository that was set up under the old push model, remove all three; that is the whole config side of the
   migration. Dropping the installer, not just the publish job, matters: with the installer left in, every release still
   attaches a `<crate_name>.rb` that nobody consumes and that can be mistaken for the published formula.
4. Use the discovered dist version from Phase B for `cargo-dist-version`.

### Phase D: Ensure Cargo.toml is ready for dist

1. Ensure `Cargo.toml` has `repository = "https://github.com/<owner>/<repo>"` — dist requires this for GitHub CI.
2. Ensure `Cargo.toml` has `description` and `homepage`. They are ordinary crate metadata. The formula the tap generates
   states a description and homepage too, but the tap sources those on its side during onboarding; nothing here feeds
   them to the tap automatically.
3. If `Cargo.toml` sets `readme` or `license-file`, they must point at `README.md` and `LICENSE`. Either field replaces
   dist's automatic file discovery for that file (see Phase J), so a value like `LICENSE-MIT` or `README.txt` puts a
   wrongly named member in the archive and the tap rejects it. Leaving them unset is fine.
4. `[profile.dist]` is added automatically by `dist init --yes` on the new-repository path in Phase E. On the
   existing-pin path nothing adds it; if it is missing, add what `dist init` writes (`inherits = "release"` and
   `lto = "thin"` on dist 0.32.0) by hand. dist runs without it, but the release binaries would then be built with the
   plain `release` profile.

### Phase E: Generate the Dist Release Workflow

`.github/workflows/release.yml` is **dist-managed** and must never be hand-edited. Which dist command produces it
depends on the case from Phase B.

New repository (no existing pin):

1. Write `dist-workspace.toml` first (Phase C).
2. Ensure `Cargo.toml` has `repository`, `description`, `homepage` (Phase D).
3. Run `dist init --yes`. It adds `[profile.dist]` to `Cargo.toml` if missing, rewrites `dist-workspace.toml` in its own
   layout (values are preserved; custom comments are dropped and replaced with dist's stock ones), and generates
   `release.yml`. The `--yes` flag auto-accepts defaults, which non-interactive runs need.

Existing pin (a migration, or any later change to `dist-workspace.toml`):

1. Edit `dist-workspace.toml` (Phase C).
2. Run `$D generate`, then `$D generate --check`, with `$D` the pinned-version binary from Phase B. Never run
   `dist init` here; it would bump the pin. `generate` leaves comments in `dist-workspace.toml` alone.

Existing pin with `allow-dirty = ["ci"]` (saltybox's shape, from Phase A):

`allow-dirty = ["ci"]` does more than tolerate a hand edit. With it set, `$D generate` exits 0 and writes nothing, and
`$D generate --check` exits 0 whatever the file contains, so the previous case's two commands silently do nothing and
the old publish job stays in `release.yml`. Regenerate by removing the `allow-dirty` line temporarily, running
`$D generate`, re-applying the repository's local patch to the fresh output (for saltybox, the `contents: read` at
workflow level with `contents: write` only on the `host` job), then restoring `allow-dirty`. Say in the PR that
`generate --check` proves nothing on this repository. If the local patch is not documented anywhere, diff the old
`release.yml` against the fresh output before discarding it; that diff is the patch.

In all cases, confirm afterwards that `release.yml` has no `publish-homebrew-formula` job and references no secret other
than `GITHUB_TOKEN`. If either is present, push-model keys survive somewhere dist reads (`dist-workspace.toml`, or the
`metadata.dist` tables in `Cargo.toml`), or `allow-dirty` prevented the rewrite; fix the cause and regenerate rather
than editing the workflow.

### Phase F: Install Linux/macOS CI Pattern

1. Create or update `.github/workflows/ci.yml` using `references/ci-linux-macos-pattern.md`.
2. Keep standard CI Linux-focused.
3. Keep a macOS job disabled in standard CI for cost control.
4. Omit Windows baseline jobs unless explicitly requested.

### Phase G: Add Release Plan Test Workflow

This file is NOT generated by `dist init`. It is a manually-maintained reusable workflow that the dist-generated
`release.yml` calls via `plan-jobs = ["./release-plan-tests"]`. Create it AFTER generating `release.yml` (Phase E) so
you can verify `release.yml` references it correctly.

1. Create or update `.github/workflows/release-plan-tests.yml` using `references/release-plan-tests-template.md`.
2. Run Linux tests on workflow call.
3. Run macOS tests only when `github.ref` is a tag ref.
4. Ensure `dist-workspace.toml` includes `plan-jobs = ["./release-plan-tests"]`.
5. Verify the generated `release.yml` contains a `custom-release-plan-tests` job that calls this workflow.

### Phase H: Enforce Conventional Commit PR Titles and Changelog Decision Tags

1. Create or update `.github/workflows/conventional-commit-pr-title.yml` using
   `references/conventional-commit-pr-title-workflow.md`.
2. This workflow contains two jobs:
   - `conventional-commit`: validates PR title against allowed Conventional Commit types.
   - `changelog-decision`: validates PR body contains exactly one of `changelog: include` or `changelog: skip`.
3. Enforce these allowed types:
   - `feat`, `fix`, `docs`, `doc`, `perf`, `refactor`, `style`, `test`, `chore`, `ci`, `revert`
4. Keep scope optional.
5. Enforce classification policy in repository docs:
   - Type must reflect user-visible behavior, not implementation activity.
   - CLI interface/behavior changes (commands, flags/options, arguments, output contract, exit codes, documented usage)
     must be `feat`, `fix`, or `perf` (use `!` when breaking), not `refactor`.
   - `refactor`, `style`, `test`, `chore`, `ci`, `docs`, and `doc` are for non-user-visible changes only.
6. Update `CLAUDE.md` to require Conventional Commit style PR titles and changelog decision tags. Add a section like:

   ```
   # PR titles

   PR titles must follow [Conventional Commits](https://www.conventionalcommits.org/) style. This is enforced by CI
   and used by git-cliff for changelog generation.

   Allowed types: `feat`, `fix`, `docs`, `doc`, `perf`, `refactor`, `style`, `test`, `chore`, `ci`, `revert`.
   Scope is optional. Examples: `feat: add user login`, `fix(parser): handle empty input`.

   Type must reflect user-visible behavior, not implementation activity.
   CLI interface/behavior changes must be `feat`, `fix`, or `perf` (use `!` when breaking), not `refactor`.

   Every PR body must contain exactly one of `changelog: include` or `changelog: skip`. This is enforced by CI.
   ```

   If `CLAUDE.md` already has a section about commit messages or PR titles, extend it rather than duplicating.

### Phase I: Set Up git-cliff and Release Documentation

1. If `cliff.toml` is missing, initialize it with:
   - `git cliff --init keepachangelog`
2. If `cliff.toml` already exists, avoid replacing it with a hardcoded template unless the user explicitly requests that
   migration.
3. Customize the generated config following `references/git-cliff-and-changelog-flow.md`:
   - Replace `commit_parsers` with the robust version that checks message, body, and footer for changelog tags, uses
     case-insensitive word-boundary regexes, and matches full Conventional Commit syntax with optional scope and `!`.
   - Set `filter_unconventional = true` and `filter_commits = true`.
   - Update the body template to strip `changelog: include` / `changelog: skip` from rendered entries.
   - Use distinct group names: "Added" for feat, "Fixed" for fix, "Performance" for perf, "Reverted" for revert.
   - Include by default: `feat`, `fix`, `perf`, `revert`.
   - Skip by default: `refactor`, `style`, `test`, `chore`, `ci`, `docs`, `doc`.
   - Parser order matters: `changelog: skip` overrides first, then type-based grouping, then `changelog: include` as a
     rescue for otherwise-skipped types. The include rules must NOT come before the type rules — CI enforces a changelog
     tag in every PR body and squash merges carry it into the commit body, so an early include rule would group every
     commit merged under the convention into "Changed".
   - If both tags are present, `changelog: skip` wins.
4. Update `CONTRIBUTING.md` with:
   - Conventional Commit requirements for commit messages and PR titles.
   - Classification policy: type reflects user-visible behavior; CLI interface changes are never `refactor`.
   - Note that PR title enforcement and changelog decision tag validation are in
     `.github/workflows/conventional-commit-pr-title.yml`.
   - Every PR body must contain exactly one of `changelog: include` or `changelog: skip`.
   - Changelog generation uses git-cliff and root `CHANGELOG.md`.
   - Override tag behavior for `changelog: include` / `changelog: skip`.
   - A **Release Notes** section documenting the `release-notes/X.Y.Z.md` convention: custom release commentary can be
     added by creating this file before cutting a release; its contents are inserted into `CHANGELOG.md` between the
     version heading and the auto-generated entries as part of the changelog generation step.
   - An agent-centric **Releasing** section using the content from `references/release-checklist.md`. This section is
     written as instructions for an AI agent so that a user can say "cut a release" and the agent guides them through
     the entire version bump, changelog, PR, merge, tag, and release watch flow, ending with the hand-off of the new tag
     to the tap. The checklist's preflight section carries the archive-contract check and the no-push rule; keep both
     when adapting it to a repository.
5. Update `CLAUDE.md` with a Releasing section that tells agents to follow CONTRIBUTING.md:

   ```
   # Releasing

   When the user asks to "make a release" or "cut a release", follow the Releasing section of `CONTRIBUTING.md`.
   ```

   If `CLAUDE.md` already has a releasing section, update it rather than duplicating.

### Phase J: Confirm the Release Satisfies the Tap's Contract

The tap pulls; this phase checks that what the repository publishes is what the tap will accept, and documents the
hand-off. Nothing here writes to the tap.

1. Run `$D plan` and check each of the four `<crate_name>-<target>.tar.xz` archives: its `[bin]` line names exactly
   `<crate_name>`, its `[misc]` line lists exactly `CHANGELOG.md`, `LICENSE`, `README.md`, and no `.rb` artifact appears
   anywhere in the plan. The `[checksum]` line under each archive, the `source.tar.gz`, `sha256.sum`, and
   `<crate_name>-<target>-update` entries are separate release assets the tap never fetches; they are not violations. On
   a brand-new repository `CHANGELOG.md` does not exist until the first release PR creates it, so `[misc]` is one file
   short until then; note that rather than adding a placeholder, and re-check on the first release.

   How dist fills `[misc]`, for when the listing is wrong: it scans the package directory, then the workspace root, by
   name prefix. Every file starting with `LICENSE` or `UNLICENSE` is packed; one `README*` and one `CHANGELOG*` or
   `RELEASES*` is picked, with no documented preference when several exist; a `license-file` or `readme` field in
   `Cargo.toml` replaces the scan for that file. So the repository needs exactly one `LICENSE`, one `README.md`, and one
   `CHANGELOG.md`, with no `LICENSE-MIT`, `CHANGELOG.txt`, `RELEASES.md`, or similar alongside. A missing file leaves
   the archive short and a wrong or extra one changes the member list; the tap rejects both.
2. Sweep the repository for push-model leftovers. The expected hits are the CONTRIBUTING prose written in Phase I and,
   where someone added one after generation, a comment in `dist-workspace.toml` naming the tap; anything else is a
   leftover:

   ```sh
   rg -n -i 'HOMEBREW_TAP_TOKEN|homebrew-dist-tap|publish-jobs|publish-homebrew|installers = \["homebrew"\]' \
     --glob '!lore/**' --glob '!target/**' --glob '!CHANGELOG.md' .
   ```

   The trailing `.` is not decoration: without a path, `rg` reads stdin when stdin is not a terminal, and under an agent
   harness that means hanging until the tool call times out.

3. Confirm `README.md` documents installation as `brew install scode/dist-tap/<crate_name>`; add the line only on a new
   repository (a migration leaves `README.md` alone, and treeward and saltybox already had it). On a brand-new tool that
   line is a promise the tap has not kept yet: the formula exists only after the tap owner onboards the tool there. Say
   so to the user rather than implying `brew install` works the moment the first release is tagged.
4. Tell the user what remains outside this repository, in one place at the end of the setup report:
   - a new tool has to be onboarded in `scode/homebrew-dist-tap` (per-tool updater, `pull.toml` entry, formula test)
     before any release can be pulled, and the tap's `update` command refuses tools it does not know;
   - a migrated tool's `HOMEBREW_TAP_TOKEN` repository secret is now unused but is not removed by this setup. Secrets
     live in repository settings, not the tree, so a PR cannot remove it, and deleting this repository's copy is
     separate from revoking the underlying token, which other tools may still use. Both are the owner's call;
   - on a migration, the repository change should land before the tap enables the tool's pull entry, so the formula
     never has two writers.

## Verification Checklist

Run these checks after setup. The negative checks matter as much as the positive ones; a push-model key that survived
regenerates the publish job on the next `dist generate`.

1. `rg -n '^cargo-dist-version = "' dist-workspace.toml && rg -n '^installers = \[\]' dist-workspace.toml && rg -n '^plan-jobs = \["./release-plan-tests"\]' dist-workspace.toml`
   (three separate checks; one alternation would pass on any single hit)
2. `! rg -n '^\s*(tap|publish-jobs)\s*=|^\s*installers\s*=.*homebrew' dist-workspace.toml Cargo.toml` (keys, not words,
   so a comment that names the tap does not trip it; `Cargo.toml` is included because dist also reads its
   `metadata.dist` tables)
3. `rg -n '^\[profile\.dist\]' Cargo.toml`
4. `rg -n 'custom-release-plan-tests' .github/workflows/release.yml`
5. `! rg -n 'publish-homebrew-formula|HOMEBREW_TAP_TOKEN|homebrew-dist-tap' .github/workflows/release.yml`
6. `rg -n 'test-linux|test-macos' .github/workflows/release-plan-tests.yml`
7. `rg -n 'action-semantic-pull-request|changelog-decision|github-script' .github/workflows/conventional-commit-pr-title.yml`
8. `rg -n 'Conventional Commits|PR titles|Releasing|CONTRIBUTING.md' CLAUDE.md`
9. `rg -n 'conventional_commits = true' cliff.toml`
10. `rg -n 'git-cliff --tag|CHANGELOG\.md|Conventional Commits|cut a release|bump|release-notes/' CONTRIBUTING.md`
11. `rg -n 'homebrew-dist-tap' CONTRIBUTING.md` (at least the hand-off step in Releasing)
12. `! rg -n 'HOMEBREW_TAP_TOKEN|formula publish' CONTRIBUTING.md`
13. `rg -n 'brew install scode/dist-tap/' README.md`
14. `$D generate --check` exits 0 (`$D` being the pinned dist binary from Phase B, or the installed `dist` on a new
    repository), and `$D plan` shows no `.rb` artifact. On a repository with `allow-dirty = ["ci"]` the first half is
    vacuous, since `--check` passes anything there; rely on item 5 and the plan output instead.

What these checks cannot prove: byte-for-byte conformance of a built archive to the tap's contract. That is only
established on the next tagged release, by the tap's `cargo xtask update <crate_name> --tag vX.Y.Z --dry-run`. Say that
in the setup report instead of claiming end-to-end coverage.

## Resources

Use these files to avoid rewriting long templates:

- `references/dist-workspace-template.md`
- `references/ci-linux-macos-pattern.md`
- `references/release-plan-tests-template.md`
- `references/conventional-commit-pr-title-workflow.md`
- `references/git-cliff-and-changelog-flow.md`
- `references/release-checklist.md`
