---
name: scode-chores
description: Run periodic repository maintenance chores and prepare one stacked PR per approved chore. Use when the user explicitly invokes /scode-chores or $scode-chores to check dependency-lock updates, dprint plugin updates, or GitHub Actions version updates.
---

# Scode Chores

Run routine dependency and tooling update chores, then offer the user the PRs that are worth creating. This is not a
general modernization pass. It should only make boring update PRs whose main purpose is keeping existing dependencies,
formatting plugins, and GitHub Actions current.

## Workflow

1. Detect which chores apply from the repository root.
2. For each applicable chore, run the smallest update command that can prove whether a real diff exists.
3. Record the exact diff summary, then restore the worktree before trying the next chore. Do not leave discovery diffs
   piled on top of each other.
4. Present the full option list to the user:
   - applicable chores with an upgrade diff
   - applicable chores that were already current
   - chores that were skipped because the project does not use that tool
   - chores that were blocked because a required command or external lookup failed
5. Ask which upgrade PRs to create. If the user already said to do all available chores, treat every chore with a real
   diff as approved.
6. Use `$jjstack` for the approved chores. By default, create one PR per chore, in the checklist order below. Do not
   combine Rust, dprint, and GitHub Actions updates into a single PR unless the user explicitly asks.
7. Recreate each approved chore's diff in its own stack entry, run the relevant verification for that chore, and let
   `$jjstack` create or update the stacked PRs.
8. Summarize which PRs were created, which chores were already current, which were skipped, and which were blocked.

Before starting discovery, inspect VCS state. If the worktree has unrelated user changes, do not overwrite or revert
them. Either use a temporary worktree for discovery or record enough state to revert only the files changed by the
current chore.

## Checklist

### 1. Update Rust lockfile dependencies

**Detect:** The repository has a `Cargo.toml` at the root. Treat this chore as PR-worthy only when `Cargo.lock` is
tracked or otherwise intentionally committed.

**Skip if:** There is no Rust project, or the project does not commit a `Cargo.lock`.

**Discovery command:**

```sh
cargo update
```

**PR condition:** `Cargo.lock` changed.

**Why:** `cargo update` keeps the existing semver constraints but refreshes the lockfile to newer compatible crate
versions. That makes dependency drift visible in a small, reviewable PR instead of bundling it into unrelated work.

**Verify:** Run the repository's normal Rust checks when they are documented. If there is no repo-specific guidance, run
`cargo test`. For workspaces where that is too expensive or unavailable, run the narrowest command that proves the
updated lockfile still resolves, and report the limitation.

### 2. Update dprint WASM plugins

**Detect:** The repository has a root dprint config such as `dprint.json`, `dprint.jsonc`, `.dprint.json`, or
`.dprint.jsonc`.

**Skip if:** There is no root dprint config.

**Discovery command:**

```sh
dprint config update
```

If the project uses a different dprint config path, use the project's path instead of assuming the root file.

**PR condition:** The dprint config changed, normally under the `plugins` list.

**Why:** dprint pins formatter plugins by URL. Updating those URLs keeps formatting behavior current while making any
formatter-version change explicit.

**Verify:** Run `dprint check`. If the plugin update changes formatting behavior, run `dprint fmt`, include the
formatting diff in the same dprint PR, then run `dprint check` again.

### 3. Update GitHub Actions versions

**Detect:** The repository has `.github/workflows/*.yml` or `.github/workflows/*.yaml`, and at least one workflow step
has a `uses:` value that points at a GitHub Action.

**Skip if:** There are no GitHub Actions workflows, or no external actions are referenced.

**Discovery process:**

- Find action references with `rg 'uses:\s*[^./][^@]+@' .github/workflows`.
- Ignore local actions (`./...`), Docker image actions (`docker://...`), and non-version refs that are intentionally
  branches or SHAs unless the user asks to pin or change them.
- For each version-tagged action, check the upstream repository's current stable tag using GitHub releases/tags or an
  available action-update tool. Prefer official GitHub data over third-party indexes.
- Update to the newest stable version tag. Avoid prereleases unless the existing ref is already a prerelease or the user
  explicitly asks for prereleases.

**PR condition:** One or more workflow `uses:` refs changed.

**Why:** Actions are executable CI dependencies. Keeping them current picks up security fixes and platform compatibility
updates without mixing CI dependency bumps into feature work.

**Verify:** Read the workflow diff and confirm every changed `uses:` ref points at an existing upstream tag. Run any
local workflow linter the repository already uses. If no local validation exists, report that GitHub Actions validation
will happen in CI after the PR is opened.

## PR Boundaries

Use these default commit and PR scopes:

- Rust dependency chore: `chore: update rust dependencies`
- dprint plugin chore: `chore: update dprint plugins`
- GitHub Actions chore: `ci: update github actions`

Keep each approved chore as one stack entry even when it changes multiple files. For example, a dprint plugin update and
the formatting changes caused by that update belong in the same dprint PR.

If one approved chore depends on another, say so before creating PRs and put the prerequisite lower in the stack.
Otherwise use the checklist order.
