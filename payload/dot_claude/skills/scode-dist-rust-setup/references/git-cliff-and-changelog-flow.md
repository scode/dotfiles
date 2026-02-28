# git-cliff and Changelog Flow

Use `git cliff --init keepachangelog` as the initialization source of truth so the generated config tracks upstream tool
evolution.

## Initialize cliff.toml

Run this once per repository history when `cliff.toml` does not exist:

```bash
git cliff --init keepachangelog
```

## Existing cliff.toml Handling

- If `cliff.toml` already exists, keep it unless the user explicitly requests re-initialization.
- Do not replace existing config with a static template by default.
- Keep configuration compatible with Conventional Commit parsing because release/changelog policy depends on it.
- Default changelog policy to user-visible entries only:
  - Include by default: `feat`, `fix`, `perf`, `revert`.
  - Skip by default: `refactor`, `style`, `test`, `chore`, `ci`, `docs`, `doc`.
  - Parse override tags first: `changelog: include` forces inclusion and `changelog: skip` forces exclusion.
  - If both tags are present, `changelog: skip` wins.

## CONTRIBUTING.md Content Requirements

Keep these sections explicit in CONTRIBUTING:

- Commit and PR titles must follow Conventional Commit headers.
- Type reflects user-visible behavior. CLI interface/behavior changes are never `refactor`; use `feat`, `fix`, or `perf`
  (`!` when breaking).
- PR title enforcement is implemented in `.github/workflows/conventional-commit-pr-title.yml`.
- Changelog generation uses git-cliff and root `CHANGELOG.md`.
- Changelog override tags: `changelog: include` and `changelog: skip` (skip wins when both are present).
- Agent-centric Releasing section (see Phase I in `SKILL.md` for the template).
