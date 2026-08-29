---
name: scode-modernize
description: Scan a repository for known deprecated or outdated patterns and propose or apply the modern equivalent. Use when the user explicitly invokes /scode-modernize or $scode-modernize to audit and modernize project conventions incrementally.
---

# Scode Modernize

Scan the current project for known deprecated or outdated patterns. For each pattern found, replace it with the modern
equivalent. Report what was changed and what was already clean.

## Workflow

1. Detect project type (Rust, etc.) from the repository root.
2. Walk the checklist below. For each item, check whether the pattern exists. Collect all findings.
3. Present the full list of findings to the user and ask which ones to address.
4. Apply replacements only for the items the user approves. By default, treat each approved item as its own `$jjstack`
   PR. If the user gives an explicit order, apply the items in that order; otherwise use the checklist order.
5. After all approved items are processed, summarize: which items were fixed, which were skipped, which were already
   clean.

## Checklist

### 1. Add formatting, clippy, and test CI jobs (Rust projects)

**Detect:** The project has a `Cargo.toml` at the repository root (or is a Cargo workspace). Check
`.github/workflows/*.yml` for existing CI jobs that run `cargo fmt` / `rustfmt`, `cargo clippy`, and `cargo test`. A job
counts as present only if it runs the check as a dedicated job (not a step buried inside another job).

**Skip if:** Dedicated formatting, clippy, and test jobs all already exist as separate jobs.

**Why:** Formatting, lint, and test checks catch issues early. Running them as separate jobs means they report failures
in parallel, making CI feedback faster and easier to triage. Bundling them into a single job delays feedback and
obscures which check failed.

**Replace with:**

- If no CI workflow exists at all, create `.github/workflows/ci.yml`.
- Add a `fmt` job that checks formatting. Use `actions-rust-lang/setup-rust-toolchain@v1` to install the `rustfmt`
  component, then run `cargo fmt --all -- --check`:

```yaml
fmt:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        components: rustfmt
    - run: cargo fmt --all -- --check
```

- Add a `clippy` job that runs clippy. Use `actions-rust-lang/setup-rust-toolchain@v1` to install the `clippy`
  component, then run `cargo clippy`:

```yaml
clippy:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-
    - uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        components: clippy
    - run: cargo clippy
```

- Add a `test` job that runs the test suite:

```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/cache@v4
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: |
          ${{ runner.os }}-cargo-
    - run: cargo test
```

- If `Cargo.lock` is not committed (library crates), use `hashFiles('**/Cargo.toml')` for the cache key instead.
- If existing CI already has formatting, clippy, or tests as steps within another job, extract them into their own jobs
  and remove the steps from the original job.
- Only use official actions: `actions/checkout`, `actions/cache`, and `actions-rust-lang/setup-rust-toolchain`. Do not
  use `actions-rs/*` or third-party community actions.
- Every Rust CI job that compiles code (clippy, test, build — but not fmt, which only runs rustfmt) must include cargo
  caching with `actions/cache@v4`. Use the same cache configuration shown in the clippy example above, placed
  immediately after checkout.
- When auditing an existing workflow, check that every job which runs `cargo build`, `cargo test`, or `cargo clippy` has
  a cache step. Add one if missing.

**Verify:** The workflow file has separate `fmt`, `clippy`, and `test` jobs. Every job that compiles Rust code has cargo
caching. `act` or a manual read confirms each job runs the expected command.

### 2. Switch GitHub Actions workflows to Ubicloud runners

**Detect:** The project has GitHub Actions workflows under `.github/workflows/` and at least one job uses a GitHub
hosted Ubuntu runner such as `ubuntu-latest`, `ubuntu-24.04`, or `ubuntu-22.04` instead of an Ubicloud runner.

**Skip if:** There are no GitHub Actions workflows, every job already uses an Ubicloud runner, or the workflow clearly
needs a GitHub-hosted runner for a documented reason.

**Why:** Ubicloud runners are a standard part of the baseline CI setup. They are compatible with GitHub runner images
for ordinary Linux CI, and the short label keeps the workflow independent of a specific CPU size until the project has a
reason to care.

**Replace with:**

- By default, do the same migration as <https://github.com/scode/ferricode/pull/6>: replace each GitHub-hosted Ubuntu
  runner with `runs-on: ubicloud` and leave the rest of the workflow alone.
- Preserve intentional non-Linux jobs. Do not rewrite macOS, Windows, self-hosted, or container-only jobs unless the
  user explicitly asks for that.
- If the user asks to use Ubicloud with a custom runner, present the full available runner list below and ask which
  label to use before editing. The list was pulled from Ubicloud's runner type documentation and linked
  `github_runner_labels.yml` on 2026-06-27; do not browse during the normal modernization run just to refresh it.
- Do not offer deprecated arm label aliases unless the user specifically asks about backwards compatibility.

Available Ubicloud runner labels:

- `ubicloud` (alias for `ubicloud-standard-2-ubuntu-2404`)
- `ubicloud-arm` (alias for `ubicloud-standard-2-arm-ubuntu-2404`)
- `ubicloud-standard-2`, `ubicloud-standard-4`, `ubicloud-standard-8`, `ubicloud-standard-16`, `ubicloud-standard-30`,
  `ubicloud-standard-60`
- `ubicloud-standard-2-ubuntu-2204`, `ubicloud-standard-4-ubuntu-2204`, `ubicloud-standard-8-ubuntu-2204`,
  `ubicloud-standard-16-ubuntu-2204`, `ubicloud-standard-30-ubuntu-2204`, `ubicloud-standard-60-ubuntu-2204`
- `ubicloud-standard-2-ubuntu-2404`, `ubicloud-standard-4-ubuntu-2404`, `ubicloud-standard-8-ubuntu-2404`,
  `ubicloud-standard-16-ubuntu-2404`, `ubicloud-standard-30-ubuntu-2404`, `ubicloud-standard-60-ubuntu-2404`
- `ubicloud-standard-2-ubuntu-2604`, `ubicloud-standard-4-ubuntu-2604`, `ubicloud-standard-8-ubuntu-2604`,
  `ubicloud-standard-16-ubuntu-2604`, `ubicloud-standard-30-ubuntu-2604`, `ubicloud-standard-60-ubuntu-2604`
- `ubicloud-premium-2`, `ubicloud-premium-4`, `ubicloud-premium-8`, `ubicloud-premium-16`, `ubicloud-premium-30`
- `ubicloud-premium-2-ubuntu-2204`, `ubicloud-premium-4-ubuntu-2204`, `ubicloud-premium-8-ubuntu-2204`,
  `ubicloud-premium-16-ubuntu-2204`, `ubicloud-premium-30-ubuntu-2204`
- `ubicloud-premium-2-ubuntu-2404`, `ubicloud-premium-4-ubuntu-2404`, `ubicloud-premium-8-ubuntu-2404`,
  `ubicloud-premium-16-ubuntu-2404`, `ubicloud-premium-30-ubuntu-2404`
- `ubicloud-premium-2-ubuntu-2604`, `ubicloud-premium-4-ubuntu-2604`, `ubicloud-premium-8-ubuntu-2604`,
  `ubicloud-premium-16-ubuntu-2604`, `ubicloud-premium-30-ubuntu-2604`
- `ubicloud-standard-2-arm`, `ubicloud-standard-4-arm`, `ubicloud-standard-8-arm`, `ubicloud-standard-16-arm`,
  `ubicloud-standard-30-arm`, `ubicloud-standard-60-arm`
- `ubicloud-standard-2-arm-ubuntu-2204`, `ubicloud-standard-4-arm-ubuntu-2204`, `ubicloud-standard-8-arm-ubuntu-2204`,
  `ubicloud-standard-16-arm-ubuntu-2204`, `ubicloud-standard-30-arm-ubuntu-2204`, `ubicloud-standard-60-arm-ubuntu-2204`
- `ubicloud-standard-2-arm-ubuntu-2404`, `ubicloud-standard-4-arm-ubuntu-2404`, `ubicloud-standard-8-arm-ubuntu-2404`,
  `ubicloud-standard-16-arm-ubuntu-2404`, `ubicloud-standard-30-arm-ubuntu-2404`, `ubicloud-standard-60-arm-ubuntu-2404`
- `ubicloud-standard-2-arm-ubuntu-2604`, `ubicloud-standard-4-arm-ubuntu-2604`, `ubicloud-standard-8-arm-ubuntu-2604`,
  `ubicloud-standard-16-arm-ubuntu-2604`, `ubicloud-standard-30-arm-ubuntu-2604`, `ubicloud-standard-60-arm-ubuntu-2604`

**Verify:** Workflow jobs that previously used GitHub-hosted Ubuntu runners now use `runs-on: ubicloud`, or the
user-selected Ubicloud label when they explicitly requested a custom runner. Non-Linux jobs are unchanged.

### 3. Keep agent finish-work commands in sync with CI

**Detect:** The repository has `AGENTS.md` and/or `CLAUDE.md` instructions that tell the agent what to run before
finishing work, creating a PR, or claiming the work is done. Compare those commands against the commands actually run by
required CI workflows in `.github/workflows/*.yml`. Treat this as a finding when the command sets differ, when the same
tool is run with different arguments (for example `cargo clippy` vs. `cargo clippy --all-targets --all-features`), or
when the instructions omit a required CI signal entirely.

**Skip if:** There are no agent instruction files, no CI workflow files, or no finish-work/checks section in the agent
instructions. Also skip if the instructions already match the CI commands exactly.

**Why:** The agent should run the same checks locally that GitHub will run after a PR is opened. Near-matches are not
good enough: a missing flag can hide exactly the failure CI is about to report.

**Replace with:**

- Identify the CI jobs that gate PR health. For Rust projects, include every workflow command that runs `cargo fmt`,
  `cargo clippy`, `cargo test`, `cargo check`, `cargo build`, `dprint`, or another explicit formatter/linter/test
  command.
- Normalize only shell trivia. Compare the actual tool invocation and arguments exactly after removing harmless YAML
  wrapping, step names, and surrounding shell boilerplate. Do not treat `cargo clippy` and
  `cargo clippy --all-targets --all-features` as equivalent.
- Update the finish-work section in `AGENTS.md` (preferred) or `CLAUDE.md` so the required commands match CI exactly,
  including flags, `--` argument separators, feature flags, workspace flags, package selectors, and check-only flags.
- If both `AGENTS.md` and `CLAUDE.md` are regular files, keep them consistent with each other. If they conflict and
  there is no clear canonical file, report the conflict and ask which file should be authoritative before editing.
- If CI uses a matrix, environment variable, setup step, or script wrapper that materially changes what command runs,
  capture that faithfully in the agent instructions. Prefer the literal command the agent can run locally; if the CI
  behavior cannot be expressed as a single local command, say so explicitly in the instructions instead of inventing an
  approximation.
- If a CI command is intentionally not useful locally, do not silently drop it. Either document the reason in the
  finish-work section or report it as an unresolved modernization finding.

**Verify:** Read the final `AGENTS.md` / `CLAUDE.md` instructions and the workflow files side by side. Every local
finish-work command matches a required CI command exactly, every required CI signal is represented, and no extra local
command is described as required unless it is intentionally stricter than CI and the instructions say that.

### 4. Replace `actions-rs/*` GitHub Actions (Rust projects)

**Detect:** Any `.github/workflows/*.yml` file that references `actions-rs/` in a `uses:` field (e.g.
`actions-rs/toolchain`, `actions-rs/cargo`, `actions-rs/clippy-check`, `actions-rs/audit-check`).

**Why:** The `actions-rs` organization is unmaintained and deprecated. Ubuntu runners already ship with a recent stable
Rust toolchain, so a dedicated toolchain action is unnecessary for stable builds. Removing the dependency simplifies CI
and eliminates a supply-chain risk.

**Replace with:**

- Remove all `uses: actions-rs/*` steps.
- Replace `actions-rs/cargo` steps with direct `cargo` invocations (`run: cargo build`, `run: cargo test`, etc.).
- Replace `actions-rs/toolchain` steps — if the workflow only needs stable Rust, simply remove the step (ubuntu has it).
  If a specific toolchain or component is needed (e.g. nightly, rustfmt, clippy), use
  `actions-rust-lang/setup-rust-toolchain@v1` with appropriate `toolchain:` and `components:` inputs.
- Replace `actions-rs/clippy-check` with a plain `run: cargo clippy` step.
- Replace `actions-rs/audit-check` with `run: cargo install cargo-audit && cargo audit` or equivalent.
- Add cargo caching using `actions/cache@v4`:

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      target
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

- Place the cache step immediately after `actions/checkout`.
- If `Cargo.lock` is not committed (e.g. library crates), use `hashFiles('**/Cargo.toml')` instead.

**Verify:** `rg 'actions-rs/' .github/` returns no matches after replacement.

### 5. Canonicalize agent instructions as `AGENTS.md`

**Detect:** Check the repository root for `AGENTS.md` and `CLAUDE.md`.

**Skip if:** `AGENTS.md` exists and is a regular file. It is fine if `CLAUDE.md` is missing or is already a symlink to
`AGENTS.md`.

**Also skip if:** Neither `AGENTS.md` nor `CLAUDE.md` exists. Some projects have no agent instructions yet, and this
modernization should not create a blank file just to satisfy a naming preference.

**Why:** `AGENTS.md` is the canonical cross-agent instruction file. Keeping `CLAUDE.md` as a symlink preserves
compatibility with older Claude-specific tooling without forcing people to maintain two copies of the same rules.

**Replace with:**

- If `AGENTS.md` exists and is a symlink, replace it with a regular file containing the symlink target's current
  content. Do not leave the canonical instruction file as a symlink.
- If `AGENTS.md` does not exist and `CLAUDE.md` exists, move `CLAUDE.md` to `AGENTS.md`, then create `CLAUDE.md` as a
  symlink to `AGENTS.md`.
- If both files exist as regular files, do not guess which one is authoritative. Report the conflict and ask the user
  which content should become canonical before changing either file.
- If `CLAUDE.md` exists but is already a symlink somewhere other than `AGENTS.md`, report that explicitly before
  changing it. The target may be intentional project-specific wiring.

**Verify:** `test -f AGENTS.md && test ! -L AGENTS.md` succeeds when either agent instruction file exists. If
`CLAUDE.md` exists after the change, `test -L CLAUDE.md && test "$(readlink CLAUDE.md)" = "AGENTS.md"` succeeds.

### 6. Remove architecture-overview bloat from `CLAUDE.md` / `AGENTS.md`

**Detect:** A `CLAUDE.md` or `AGENTS.md` at the repository root (or `.claude/` directory) whose content is predominantly
architecture overview — descriptions of what each directory contains, how modules relate to each other, summaries of the
tech stack, or restating what is already obvious from reading the code and repo structure.

**Why:** These files should contain _intent and preferences_ that an agent cannot infer from the project itself: coding
conventions, workflow rules, things to avoid, non-obvious constraints, and domain-specific decisions. Architecture
overviews are redundant — an agent can read the code — and they rot as the code evolves, becoming actively misleading.

**Replace with:**

- Remove sections that merely describe the project structure, list directories and their purposes, summarize the tech
  stack, or restate what `README.md` already covers.
- Keep any sections that express preferences, constraints, workflow instructions, or non-obvious conventions.
- Keep required checks and commands (e.g. "run `cargo fmt`", "run `cargo test`", "run `cargo clippy`"). These are
  intentional guardrails, not architecture descriptions, even if they appear alongside structural overviews.
- If the entire file is architecture overview with no preference/intent content, delete it.
- If only some sections are bloat, remove those sections and keep the rest.

**Verify:** Read the resulting file (if it still exists) and confirm every remaining section expresses an intent,
preference, or constraint that cannot be trivially inferred from the repository contents.

### 7. Migrate from `log` crate to `tracing` (Rust projects)

**Detect:** `Cargo.toml` (including workspace and member `Cargo.toml` files) lists `log` as a dependency, or the source
code uses `log::info!`, `log::debug!`, `log::warn!`, `log::error!`, `log::trace!`, or the corresponding unqualified
macros imported from `log`. Also detect `env_logger`, `simple_logger`, `pretty_env_logger`, `fern`, or `simplelog` as
dependencies — these are `log`-ecosystem logger implementations that should be replaced.

**Skip if:** No logging dependency (`log`, `tracing`, or any logger implementation) exists in any `Cargo.toml` and no
logging macros are used in the source code. The project simply doesn't do logging — leave it alone.

**Why:** The `tracing` crate is the modern Rust logging/diagnostics standard. It is a superset of `log` — all `log`
macros (`info!`, `debug!`, etc.) have direct `tracing` equivalents with the same syntax. `tracing` additionally supports
structured fields, spans for contextual diagnostics, and `async`-aware instrumentation. The ecosystem has converged on
`tracing`; most major libraries and frameworks (tokio, axum, etc.) emit `tracing` events natively.

**Replace with:**

- In `Cargo.toml`: replace `log` with `tracing`. Remove `env_logger` / `simple_logger` / `pretty_env_logger` / `fern` /
  `simplelog` and add `tracing-subscriber` instead.
- In source files: replace `use log::{...}` with `use tracing::{...}`. The macro names (`info!`, `debug!`, `warn!`,
  `error!`, `trace!`) are identical — most call sites need only the import change.
- For logger initialization (e.g. `env_logger::init()`), replace with a `tracing-subscriber` setup. A minimal
  equivalent:
  ```rust
  tracing_subscriber::fmt::init();
  ```
  If the project was using `env_logger` with `RUST_LOG` filtering, `tracing-subscriber`'s `EnvFilter` provides the same
  behavior:
  ```rust
  use tracing_subscriber::EnvFilter;
  tracing_subscriber::fmt()
      .with_env_filter(EnvFilter::from_default_env())
      .init();
  ```
- If the project has library crates that use `log` but no logger initialization, just swap the dependency and imports —
  library crates should not initialize a subscriber.
- If any dependency emits `log` events and the project wants to capture them, add the `tracing-log` bridge or enable
  `tracing`'s `log` feature (`tracing = { version = "...", features = ["log"] }`).

**Verify:** `rg 'use log' src/` and `rg '\blog\b' Cargo.toml` return no matches. `cargo check` succeeds.

### 8. Add `dprint` formatting with checksum-pinned plugins (projects with JSON, TOML, or Markdown files)

**Detect:** The project has any `.json`, `.toml`, or `.md` files. Check whether `dprint.json` already exists at the
repository root, but do not treat an existing config as "already done". In an existing config, inspect every entry in
`plugins`: a remote plugin — an `https://` URL or an `npm:` specifier — that does not end in `@<64 hex characters>` is a
finding, however precisely its filename or specifier names a version. `https://plugins.dprint.dev/json-0.23.0.wasm` is
unpinned; `https://plugins.dprint.dev/json-0.23.0.wasm@3e1f…443c` and `npm:@dprint/json@0.23.0@1bf6…1bc3` are pinned.

**Skip if:** `dprint.json` exists, every remote plugin entry carries a checksum, the core plugins for the file types
present are all configured, and CI runs `dprint check`.

**Why:** `dprint` enforces consistent formatting for JSON, TOML, and Markdown files. Without it, formatting drift
accumulates across contributors and tools. It is fast, pluggable, and easy to add to CI.

The checksum matters because a version in a URL pins a name, not bytes. Nothing stops a later download from the same URL
returning different content — a replaced release asset, a compromised host, a cache problem — and without a checksum the
formatter CI trusts can change with no diff to review. These are Wasm plugins, so the sandbox limits what a bad one can
do to the host, but it still reads every file it formats and decides what comes back. With a checksum, `dprint` refuses
a download whose bytes do not match (exit code 12, "The checksum did not match the expected checksum"). That
verification happens at download time only: a plugin already in the local cache is not re-hashed, so the pin is what
protects fresh environments — CI above all — rather than a developer's warm cache.

**Replace with:**

- Check the installed `dprint` first. `dprint add --checksum` exists from dprint 0.55.0 onwards; older binaries accept
  `dprint add` but reject `--checksum` as an unknown argument, and have no other way to pin. Feature-detect rather than
  trusting the version string: `dprint add --help | grep -q -- --checksum`. If the flag is missing, do not fall back to
  unpinned URLs: either upgrade `dprint` (the install script at `https://dprint.dev/install.sh` puts a current binary
  wherever `DPRINT_INSTALL` points, so this needs no system change) or pin the entries by hand as described below, which
  works on any version because the checksum is just part of the plugin string.

- If `dprint.json` does not exist yet, create it at the repository root with the desired formatter settings and an empty
  plugin list, then let `dprint` fill the list with pinned, current plugins. Do not hand-write plugin entries from this
  skill; they would be stale.

```json
{
  "json": {
  },
  "markdown": {
    "lineWidth": 120,
    "newLineKind": "lf",
    "textWrap": "always"
  },
  "toml": {
  },
  "excludes": [
    "**/*-lock.json"
  ],
  "plugins": []
}
```

```sh
dprint add --checksum json markdown toml
```

- If `dprint.json` already exists, keep the project's formatter settings and plugin order. Add any missing core plugin
  with `dprint add --checksum <name>`. For each existing remote entry without a checksum, pin it in place at its current
  version — do not upgrade it as a side effect of pinning; an upgrade is a separate decision for the user:
  - For an `https://…wasm` entry, the checksum is the SHA-256 of the file: `curl -fsSL <url> | sha256sum`, then edit the
    entry to `<url>@<sha256>`.
  - For an `npm:` entry, the checksum is not the hash of the Wasm file, so compute it with `dprint` itself: in a scratch
    directory, `dprint add --checksum npm:@dprint/json@0.23.0` (the same version the project has) writes the pinned form
    to a throwaway config; copy the checksum into the project's entry.
  - Edit in place rather than running `dprint add --checksum <url>` against the project config: when the unpinned entry
    is already present, `add` appends a pinned duplicate instead of replacing it, which changes plugin order and leaves
    the unpinned entry behind.

- Offer `dprint config update` as a separate, user-approved step, not as part of pinning. It upgrades every plugin to
  the latest version and, for entries that already carry a checksum, writes the new version's checksum (a pinned
  `https://` URL comes back as a pinned `npm:` specifier on current versions). It does not add checksums to entries that
  lack them: an unpinned URL is upgraded to an unpinned specifier. So pin first, then update if the user wants the
  upgrade, and re-check the pins afterwards either way.

- If the project has a `.github/workflows/ci.yml` (or similar CI workflow), add a `dprint` job:

```yaml
dprint:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - name: Check formatting with dprint
      uses: dprint/check@v2.3
```

- Run `dprint fmt` to fix any existing formatting issues.

**Verify:** Every remote plugin entry in `dprint.json` ends in `@<64 hex characters>`; for example
`grep -E '"(https?://|npm:)[^"]*"' dprint.json | grep -v -E '@[0-9a-f]{64}"'` prints nothing. Then
`dprint clear-cache && dprint check` passes — clearing the cache is what makes `check` actually download and verify
against the pins instead of reusing already-cached plugins. If `dprint config update` was run, confirm afterwards that
no entry lost its checksum. If CI was updated, confirm the `dprint` job exists in the workflow file.

### 9. Add conventional commit instructions to agent config (projects with `CLAUDE.md` or `AGENTS.md`)

**Detect:** The project has a `CLAUDE.md` or `AGENTS.md` (at the repo root or in `.claude/`) that does not already
contain conventional commit guidance (search for "conventional commit" case-insensitively).

**Why:** Conventional Commits give commit messages and PR titles a machine-parseable, human-scannable structure. Tools
like `git-cliff`, `release-please`, and `semantic-release` rely on the prefix to generate changelogs and pick version
bumps automatically. Even without automation, a consistent prefix makes `git log --oneline` trivially scannable.

**Replace with:**

- Add the following section to the project's `AGENTS.md` (preferred) or `CLAUDE.md`. Place it near any existing commit
  message or PR guidance. If the file already has a commit-message section, merge the conventional commit rules into it
  rather than creating a duplicate section.

```markdown
# Conventional Commits

All commit messages and PR titles must use Conventional Commit format: `<type>: <short summary>`

Allowed types: `feat`, `fix`, `docs`, `perf`, `refactor`, `style`, `test`, `chore`, `ci`, `revert`.

Append `!` after the type for breaking changes (e.g. `feat!: remove legacy endpoint`). Scope is optional.

Rules:

- Type reflects the user-visible effect, not the implementation activity. A bug fix that requires heavy refactoring is
  `fix`, not `refactor`. A new CLI flag is `feat`, not `chore`.
- The summary after the colon is lowercase, imperative mood, no trailing period.
- Keep the first line under 72 characters.
```

**Verify:** The target file contains a conventional commit section. Read it back and confirm the types list and rules
are present.

### 10. Add a PR base guard for stacked workflows (GitHub Actions projects)

**Detect:** The project uses GitHub Actions and allows stacked PRs, but has no required check that rejects PRs whose
base branch is anything other than the repository's main integration branch.

**Skip if:** There is already a GitHub Actions workflow or equivalent required status check that fails PRs when
`github.base_ref` is not the expected integration branch.

**Why:** GitHub does not understand stacked PR ancestry. If a child PR is merged through the web UI, GitHub merges it
into its parent branch, and a later squash merge of the parent hides the child commit as a separate history entry. A
required base-branch check makes that mistake obvious before the merge button is usable.

**Replace with:**

- Add a dedicated workflow such as `.github/workflows/pr-base.yml`.
- Trigger it on all pull requests, not only pull requests targeting `main`. A `pull_request.branches: [main]` filter is
  wrong for this guard because it prevents the workflow from running on the stacked PRs it is supposed to reject.
- Fail when `github.base_ref` is not exactly `main`:

```yaml
name: PR Base

on:
  pull_request:

jobs:
  require-main-base:
    runs-on: ubuntu-latest
    steps:
      - name: Require main as PR base
        env:
          BASE_REF: ${{ github.base_ref }}
        run: |
          if [ "$BASE_REF" != "main" ]; then
            echo "PR base must be main, got '$BASE_REF'." >&2
            exit 1
          fi
```

- If the repository's integration branch is not named `main`, replace `main` with the exact branch name. Do not use a
  prefix match such as `pr/*`; the point is to allow only the integration branch and reject everything else.
- After adding the workflow, make its `require-main-base` check required in GitHub branch protection or repository
  rulesets. The workflow alone reports the problem, but it does not block merges until GitHub requires it.

**Verify:** Open or inspect a PR whose base is not `main` and confirm the `require-main-base` job fails. Confirm the
same check is configured as required for merges to `main`; otherwise the workflow is advisory only.

### 11. Fix `changelog: include` parser ordering in `cliff.toml` (git-cliff projects)

**Detect:** The project has a `cliff.toml` with a `commit_parsers` list in which a rule matching `changelog: include`
(in message, body, or footer) appears _before_ the type-based grouping rules (`^feat`, `^fix`, `^perf`, `^revert`). Also
treat it as a finding when the ordering is correct but the rationale comment above the type-grouping rules is the older
scode-dist-rust-setup wording — a claim that PR bodies or commits "always" contain a changelog tag, without scoping the
claim to commits merged under the convention.

**Skip if:** There is no `cliff.toml`, there are no `changelog: include` parser rules at all, or the include rules
already come after the type-based grouping rules and the rationale comment already matches the current template wording.

**Why:** git-cliff applies the first matching parser. In repos where CI requires every PR body to carry a
`changelog: include` / `changelog: skip` tag, squash merges copy that tag into the commit body — so for commits merged
under the convention, an early include rule matches first and forces every entry into its group (typically "Changed").
Fix commits never reach the "Fixed" group and the generated changelog misclassifies everything. This bit saltybox: every
release section came out as "### Changed" until the parsers were reordered. The stale comment wording matters too: the
comment is the only documentation of this load-bearing ordering constraint, and the old wording overstates the invariant
(pre-convention commits in migrated repos carry no tag at all).

**Replace with:**

- Reorder `commit_parsers` to: `changelog: skip` overrides first (skip must win over everything), then the type-based
  grouping rules (`feat` → Added, `fix` → Fixed, `perf` → Performance, `revert` → Reverted), then the
  `changelog: include` rules (which now only rescue commit types that would otherwise be skipped), then the type-based
  skip rule for non-user-visible types.
- The corrected block, with rationale comments, is in
  `agent-skills/scode-dist-rust-setup/references/git-cliff-and-changelog-flow.md` — keep the two in sync.
- If the ordering is already correct and only the comment wording is stale, replace the rationale comment with the
  current block from the reference file and leave the parser rules untouched. No changelog regeneration is needed in
  that case — the output does not change.
- After reordering, regenerate the changelog (`git-cliff --tag <current-tag> -o CHANGELOG.md`) and review the diff:
  entries for past releases may legitimately move between groups (that is the fix working). Watch for hand-written
  sections that regeneration drops; restore them manually.

**Verify:** In `cliff.toml`, the first `changelog\s*:\s*include` rule appears on a later line than the `^fix` grouping
rule, and the rationale comment matches the current template in the reference file. When the ordering changed,
regenerating the changelog puts `fix:` commits under "### Fixed", not "### Changed".

### 12. Declare whether the project is public or private in `AGENTS.md`

**Detect:** The repository has an `AGENTS.md` and/or `CLAUDE.md` at the root. Check whether it contains a visibility
declaration: a statement that the project is (or may become) public and must not contain personal information, or a
statement that the project is intentionally private and local/personal assumptions are fine. Match on intent, not exact
wording — an existing paragraph that clearly makes either declaration counts.

**Skip if:** Either declaration is already present. Also skip if neither `AGENTS.md` nor `CLAUDE.md` exists; do not
create an instruction file only to hold this statement.

**Why:** Whether usernames, hostnames, local paths, and similar environment details are allowed in a repo depends
entirely on whether the repo will ever be public, and an agent cannot infer that from the code. Without an explicit
statement the agent either leaks personal details into something that later gets published, or wastes effort scrubbing a
repo that was never going to leave the machine. The declaration has to be made by the user; there is no safe default to
guess.

**Replace with:**

- Ask the user which of the two applies. Do not pick one on their behalf, and do not infer it from the presence of a
  GitHub remote or a license file — a public remote today does not mean personal details were intended to be there, and
  a private remote does not mean it will stay private.
- Add the chosen statement, verbatim, to `AGENTS.md` (preferred) or `CLAUDE.md`, near the top or alongside other
  project-wide constraints:

  For public or possibly-public projects:

  ```markdown
  This project is either public now, or may become public in the future. No content in this project should contain
  personal information such as personal usernames, hostnames, details about the local environments, etc.
  ```

  For intentionally private projects:

  ```markdown
  This project is intentionally and definitely private. It's ok to have local/personal assumptions in the content of this
  repo.
  ```

- If the user picks the public statement, that is only the declaration. Scanning the repo for existing personal details
  is out of scope for this item; mention it as a possible follow-up rather than doing it unasked.

**Verify:** The instruction file contains exactly one of the two declarations. Read it back and confirm it matches the
user's answer.
