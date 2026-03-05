# git-cliff and Changelog Flow

Use `git cliff --init keepachangelog` as the initialization source of truth so the generated config tracks upstream tool
evolution. Then customize `commit_parsers` to match the policy below.

## Initialize cliff.toml

Run this once per repository history when `cliff.toml` does not exist:

```bash
git cliff --init keepachangelog
```

Then apply these customizations to the generated config:

### commit_parsers

Replace the default `commit_parsers` with:

```toml
commit_parsers = [
  # Override tags: "changelog: skip" forces exclusion, "changelog: include" forces inclusion.
  # When both are present, skip wins. Check message, body, and footer.
  { message = "(?i)\\bchangelog\\s*:\\s*skip\\b", skip = true },
  { body = "(?i)\\bchangelog\\s*:\\s*skip\\b", skip = true },
  { footer = "(?i)\\bchangelog\\s*:\\s*skip\\b", skip = true },
  { message = "(?i)\\bchangelog\\s*:\\s*include\\b", group = "Changed" },
  { body = "(?i)\\bchangelog\\s*:\\s*include\\b", group = "Changed" },
  { footer = "(?i)\\bchangelog\\s*:\\s*include\\b", group = "Changed" },
  # Conventional Commit types included in changelog (user-visible changes).
  { message = "^feat(\\([^\\)]+\\))?!?:", group = "Added" },
  { message = "^fix(\\([^\\)]+\\))?!?:", group = "Fixed" },
  { message = "^perf(\\([^\\)]+\\))?!?:", group = "Performance" },
  { message = "^revert(\\([^\\)]+\\))?!?:", group = "Reverted" },
  # Conventional Commit types excluded from changelog (non-user-visible).
  { message = "^(docs|doc|refactor|style|test|chore|ci)(\\([^\\)]+\\))?!?:", skip = true },
]
```

### Other settings

- Set `filter_unconventional = true` to exclude non-conventional commits.
- Set `filter_commits = true` so commits not matching any parser are excluded.

### Body template

In the body template, strip changelog tags from rendered entries:

```
- {{ commit.message | split(pat="\n") | first | replace(from="changelog: include", to="") | replace(from="changelog: skip", to="") | upper_first | trim }}\
```

## Existing cliff.toml Handling

- If `cliff.toml` already exists, keep it unless the user explicitly requests re-initialization.
- Do not replace existing config with a static template by default.
- Keep configuration compatible with Conventional Commit parsing because release/changelog policy depends on it.

## CONTRIBUTING.md Content Requirements

Keep these sections explicit in CONTRIBUTING:

- Commit and PR titles must follow Conventional Commit headers.
- Type reflects user-visible behavior. CLI interface/behavior changes are never `refactor`; use `feat`, `fix`, or `perf`
  (`!` when breaking).
- PR title enforcement and changelog decision validation are in `.github/workflows/conventional-commit-pr-title.yml`.
- Every PR body must contain exactly one of `changelog: include` or `changelog: skip`.
- Changelog generation uses git-cliff and root `CHANGELOG.md`.
- Changelog override tags: `changelog: include` and `changelog: skip` (skip wins when both are present).
- Agent-centric Releasing section (see `references/release-checklist.md` for the template).
