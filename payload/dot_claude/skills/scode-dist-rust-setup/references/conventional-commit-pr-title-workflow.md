# Conventional Commit PR Title Workflow

Use this file for `.github/workflows/conventional-commit-pr-title.yml`.

```yaml
name: Conventional Commit PR Title

on:
  pull_request:
    types:
      - opened
      - edited
      - synchronize
      - reopened

permissions:
  pull-requests: read

jobs:
  conventional-commit-pr:
    name: Conventional Commit PR
    runs-on: ubuntu-latest
    steps:
      - name: Validate PR title
        uses: amannn/action-semantic-pull-request@v5
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          types: |
            feat
            fix
            docs
            doc
            perf
            refactor
            style
            test
            chore
            ci
            revert
          requireScope: false
```

## Notes

- Keep type list aligned with Conventional Commit policy used by changelog tooling.
- Keep scope optional to avoid unnecessary PR friction.
