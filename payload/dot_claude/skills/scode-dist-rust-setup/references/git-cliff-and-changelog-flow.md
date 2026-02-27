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

## CONTRIBUTING.md Content Requirements

Keep these sections explicit in CONTRIBUTING:

- Commit and PR titles must follow Conventional Commit headers.
- PR title enforcement is implemented in `.github/workflows/conventional-commit-pr-title.yml`.
- Changelog generation uses git-cliff and root `CHANGELOG.md`.
- Agent-centric Releasing section (see Phase I in `SKILL.md` for the template).
