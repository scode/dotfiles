---
name: scode-modernize
description: Scan a software project for deprecated or outdated patterns and replace them with modern equivalents. Run on any project to catch and fix known anti-patterns. Currently targets Rust projects with GitHub Actions CI, dprint formatting for any project with JSON/TOML/Markdown files, and conventional commit instructions for projects with agent config files.
---

# Scode Modernize

Scan the current project for known deprecated or outdated patterns. For each pattern found, replace it with the modern
equivalent. Report what was changed and what was already clean.

## Workflow

1. Detect project type (Rust, etc.) from the repository root.
2. Walk the checklist below. For each item, check whether the pattern exists. Collect all findings.
3. Present the full list of findings to the user and ask which ones to address.
4. Apply replacements only for the items the user approves.
5. After all approved items are processed, summarize: which items were fixed, which were skipped, which were already
   clean.

## Checklist

### 1. Replace `actions-rs/*` GitHub Actions (Rust projects)

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

### 2. Remove architecture-overview bloat from `CLAUDE.md` / `AGENTS.md`

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

### 3. Migrate from `log` crate to `tracing` (Rust projects)

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

### 4. Add `dprint` formatting (projects with JSON, TOML, or Markdown files)

**Detect:** The project has any `.json`, `.toml`, or `.md` files. Check whether `dprint.json` already exists at the
repository root, but do not treat an existing config as "already done".

**Why:** `dprint` enforces consistent formatting for JSON, TOML, and Markdown files. Without it, formatting drift
accumulates across contributors and tools. It is fast, pluggable, and easy to add to CI.

**Replace with:**

- If `dprint.json` does not exist yet, create it at the repository root with the desired formatter settings, but do not
  hand-write versioned plugin URLs:

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

- Populate the plugin list by asking `dprint` for the current latest plugin URLs at execution time instead of copying
  pinned URLs from this skill:

```sh
dprint config add json
dprint config add markdown
dprint config add toml
dprint config update
```

- If `dprint.json` already exists, keep the project's existing dprint settings, add any missing core plugins with
  `dprint config add ...`, and still run `dprint config update` so plugin URLs upgrade when newer versions are
  available.

- `dprint config add` / `dprint config update` should leave `dprint.json` with whatever plugin versions are latest at
  the time the skill is used. Do not replace those generated URLs with older hard-coded examples.

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

**Verify:** `dprint check` passes. If `dprint.json` already existed, confirm `dprint config update` was still run and
the plugin URLs were refreshed when newer versions were available. If CI was updated, confirm the `dprint` job exists in
the workflow file.

### 5. Add conventional commit instructions to agent config (projects with `CLAUDE.md` or `AGENTS.md`)

**Detect:** The project has a `CLAUDE.md` or `AGENTS.md` (at the repo root or in `.claude/`) that does not already
contain conventional commit guidance (search for "conventional commit" case-insensitively).

**Why:** Conventional Commits give commit messages and PR titles a machine-parseable, human-scannable structure. Tools
like `git-cliff`, `release-please`, and `semantic-release` rely on the prefix to generate changelogs and pick version
bumps automatically. Even without automation, a consistent prefix makes `git log --oneline` trivially scannable.

**Replace with:**

- Add the following section to the project's `CLAUDE.md` (preferred) or `AGENTS.md`. Place it near any existing commit
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
