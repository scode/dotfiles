# PR Validation Workflow

Use this file for `.github/workflows/conventional-commit-pr-title.yml`. It combines conventional commit PR title
enforcement with changelog decision tag validation.

```yaml
name: PR

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
  conventional-commit:
    name: conventional-commit
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

  changelog-decision:
    name: changelog-decision
    runs-on: ubuntu-latest
    steps:
      - name: Validate PR body has one changelog tag
        uses: actions/github-script@v7
        with:
          script: |
            const body = context.payload.pull_request?.body ?? "";
            const include = /\bchangelog:\s*include\b/i.test(body);
            const skip = /\bchangelog:\s*skip\b/i.test(body);

            if (!include && !skip) {
              core.setFailed(
                "PR description must include either 'changelog: include' or 'changelog: skip'.",
              );
              return;
            }

            if (include && skip) {
              core.setFailed(
                "PR description includes both 'changelog: include' and 'changelog: skip'. Choose exactly one.",
              );
              return;
            }

            core.info("PR changelog decision tag validated.");
```

## Notes

- Keep type list aligned with Conventional Commit policy used by changelog tooling.
- Keep scope optional to avoid unnecessary PR friction.
- The `changelog-decision` job ensures every PR explicitly declares whether it should appear in the changelog.
- Enforce this classification rule in CONTRIBUTING/agent instructions:
  - Type reflects user-visible behavior, not implementation activity.
  - CLI interface/behavior changes (commands, flags/options, arguments, output contract, exit codes, docs usage) are
    `feat`, `fix`, or `perf` (`!` when breaking), never `refactor`.
