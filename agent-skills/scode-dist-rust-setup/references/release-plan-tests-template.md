# release-plan-tests.yml Template

Use this workflow as `.github/workflows/release-plan-tests.yml`.

```yaml
name: Release Plan Tests

on:
  workflow_call:

jobs:
  test-linux:
    runs-on: ubuntu-latest
    name: test ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Setup Rust toolchain (stable)
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          cache: true
      - name: cargo generate-lockfile
        if: hashFiles('Cargo.lock') == ''
        run: cargo generate-lockfile
      - name: cargo test --locked
        run: cargo test --locked --all-features --all-targets

  test-macos:
    if: startsWith(github.ref, 'refs/tags/')
    runs-on: macos-latest
    name: test macos-latest
    steps:
      - uses: actions/checkout@v5
      - name: Setup Rust toolchain (stable)
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          cache: true
      - name: cargo generate-lockfile
        if: hashFiles('Cargo.lock') == ''
        run: cargo generate-lockfile
      - name: cargo test --locked
        run: cargo test --locked --all-features --all-targets
```

## Notes

- Keep this as a reusable `workflow_call` job.
- Ensure dist references it via `plan-jobs = ["./release-plan-tests"]`.
