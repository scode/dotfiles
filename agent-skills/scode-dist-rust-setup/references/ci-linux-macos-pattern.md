# CI Pattern: Linux Focus + macOS Deferred

Use this baseline for `.github/workflows/ci.yml`.

```yaml
name: CI

on:
  pull_request:
    branches:
      - "**"
  push:
    branches:
      - main
      - master
      - develop

env:
  CARGO_TERM_COLOR: always

concurrency:
  group: ${{ github.workflow }}-${{ github.head_ref || github.run_id }}
  cancel-in-progress: true

jobs:
  fmt:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: rustfmt
          cache: true
      - run: cargo fmt -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          components: clippy
          cache: true
      - run: cargo clippy --all-targets --all-features -- -D warnings

  doc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          cache: true
      - run: cargo doc --no-deps --all-features
        env:
          RUSTDOCFLAGS: --cfg docsrs

  test-linux:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request' || github.ref_name == 'main'
    steps:
      - uses: actions/checkout@v5
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          cache: true
      - run: cargo generate-lockfile
        if: hashFiles('Cargo.lock') == ''
      - run: cargo test --locked --all-features --all-targets

  test-macos:
    if: ${{ false }}
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: stable
          cache: true
      - run: cargo generate-lockfile
        if: hashFiles('Cargo.lock') == ''
      - run: cargo test --locked --all-features --all-targets
```

## Notes

- Keep macOS disabled in standard CI to control runner cost.
- Require macOS in release-plan tests during tag releases.
- Add Windows only when explicitly requested.
