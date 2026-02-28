---
name: scode-modernize
description: Scan a software project for deprecated or outdated patterns and replace them with modern equivalents. Run on any project to catch and fix known anti-patterns. Currently targets Rust projects with GitHub Actions CI.
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
