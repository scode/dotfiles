# combined-sol-high: original review outputs

These are the recorded outputs, including incorrect or disputed claims. The parent assessment is in
[FINDINGS.md](FINDINGS.md). Local paths and native identifiers are replaced; finding wording is retained. Source paths
refer to the pinned saltybox commit.

## correctness-combined-p1.md

```text
- **definite — hostile in-range parameters can still force a 4 GiB, 64-pass KDF** (`src/format_v2.rs:69`, `src/format_v2.rs:153`, `src/format_v2.rs:160`, `src/format_v2.rs:351`)

  **What happens:** A saltybox2 payload controls the Argon2 memory and time costs in its unauthenticated header. `validate_params` accepts a memory cost of 4,194,304 KiB together with a time cost of 64, then `decrypt` immediately derives the key before it can authenticate the payload. That makes a minimal crafted payload sufficient to make the public decrypt API attempt a 4 GiB Argon2 allocation and process that memory for 64 passes.

  **Why it matters:** The validation is described as the barrier that prevents hostile files from becoming memory or CPU bombs, but the largest accepted values are themselves enough to exhaust ordinary machines or keep a better-provisioned process busy for a very long time. Allocation failure is especially severe here because the selected Argon2 API allocates its work buffer directly; an out-of-memory condition can abort the process rather than return the `Argon2Failure` handled by this code.

  **Suggested change:** Set decrypt-side maxima to limits the supported process can safely absorb, enforced before `derive_key`. If compatibility with unusually expensive future files is required, keep the normal untrusted-input limit low and require an explicit operator-controlled override for larger parameters rather than accepting 4 GiB and 64 passes from every file header.

- **possible — the engine round-trip test adds two unintended full-cost KDF runs** (`src/format_v2.rs:435`, `src/format_v2.rs:445`)

  **What happens:** `test_engine_roundtrip_through_armor` calls `V2Engine.encrypt`, which always uses the 256 MiB write defaults, and then decrypts the result at those defaults. The following `test_default_params_roundtrip` is documented as the one test intended to exercise the real defaults, but it performs the same full-cost encrypt/decrypt pair again.

  **Why it matters:** Rust unit tests run concurrently by default, so this redundant test can overlap its 256 MiB Argon2 allocation with the other default-cost tests in this module. Whether that becomes an out-of-memory failure or timeout depends on the runner, but it needlessly doubles this coverage's expensive KDF work and makes the suite substantially less reliable on constrained CI and developer machines.

  **Suggested change:** Consolidate the trait-level armor round trip into `test_default_params_roundtrip` by using the `V2Engine` methods there, or give the engine-path test a test-only way to use `TEST_M`, `TEST_T`, and `TEST_P`, leaving only the dedicated default-parameter test at production cost.

- **possible — the randomness test performs two more 256 MiB KDF runs** (`src/format_v2.rs:467`)

  **What happens:** `test_random_encryption_uses_fresh_salt_and_nonce` invokes the production `encrypt` function twice merely to compare the generated salt and nonce. Each invocation also runs Argon2 with the 256 MiB default even though key derivation is unrelated to the property being tested.

  **Why it matters:** This test can overlap another 256 MiB Argon2 allocation with both round-trip tests under the default parallel test harness, taking the module's possible concurrent Argon2 working set to roughly 768 MiB. Whether that exhausts a particular runner remains environment-dependent, but the test adds two expensive derivations and can make otherwise-correct tests fail for resource reasons.

  **Suggested change:** Factor the random salt/nonce path so a test can exercise it with the small test KDF parameters, or test the random-header helper directly. Preserve the production `encrypt` path's use of the write defaults while avoiding full-cost key derivation in this randomness-only test.
```

## correctness-combined-p2.md

```text
[]
Reviewed the complete second pass over the full stated scope for current commit 62d866d and found no additional correctness findings.
```

## merged.md

```text
# Correctness

- `F1 / COR-KDF-LIMITS` — **definite** — `src/format_v2.rs:69`, `src/format_v2.rs:153`, `src/format_v2.rs:160`, `src/format_v2.rs:351` — hostile in-range parameters can still force a 4 GiB, 64-pass KDF

  **What happens:** A saltybox2 payload controls the Argon2 memory and time costs in its unauthenticated header. `validate_params` accepts a memory cost of 4,194,304 KiB together with a time cost of 64, then `decrypt` immediately derives the key before it can authenticate the payload. That makes a minimal crafted payload sufficient to make the public decrypt API attempt a 4 GiB Argon2 allocation and process that memory for 64 passes.

  **Why it matters:** The validation is described as the barrier that prevents hostile files from becoming memory or CPU bombs, but the largest accepted values are themselves enough to exhaust ordinary machines or keep a better-provisioned process busy for a very long time. Allocation failure is especially severe here because the selected Argon2 API allocates its work buffer directly; an out-of-memory condition can abort the process rather than return the `Argon2Failure` handled by this code.

  **Suggested change:** Set decrypt-side maxima to limits the supported process can safely absorb, enforced before `derive_key`. If compatibility with unusually expensive future files is required, keep the normal untrusted-input limit low and require an explicit operator-controlled override for larger parameters rather than accepting 4 GiB and 64 passes from every file header.

- `F2 / COR-ENGINE-KDF-COST` — **possible** — `src/format_v2.rs:435`, `src/format_v2.rs:445` — the engine round-trip test adds two unintended full-cost KDF runs

  **What happens:** `test_engine_roundtrip_through_armor` calls `V2Engine.encrypt`, which always uses the 256 MiB write defaults, and then decrypts the result at those defaults. The following `test_default_params_roundtrip` is documented as the one test intended to exercise the real defaults, but it performs the same full-cost encrypt/decrypt pair again.

  **Why it matters:** Rust unit tests run concurrently by default, so this redundant test can overlap its 256 MiB Argon2 allocation with the other default-cost tests in this module. Whether that becomes an out-of-memory failure or timeout depends on the runner, but it needlessly doubles this coverage's expensive KDF work and makes the suite substantially less reliable on constrained CI and developer machines.

  **Suggested change:** Consolidate the trait-level armor round trip into `test_default_params_roundtrip` by using the `V2Engine` methods there, or give the engine-path test a test-only way to use `TEST_M`, `TEST_T`, and `TEST_P`, leaving only the dedicated default-parameter test at production cost.

- `F3 / COR-RANDOM-KDF-COST` — **possible** — `src/format_v2.rs:467` — the randomness test performs two more 256 MiB KDF runs

  **What happens:** `test_random_encryption_uses_fresh_salt_and_nonce` invokes the production `encrypt` function twice merely to compare the generated salt and nonce. Each invocation also runs Argon2 with the 256 MiB default even though key derivation is unrelated to the property being tested.

  **Why it matters:** This test can overlap another 256 MiB Argon2 allocation with both round-trip tests under the default parallel test harness, taking the module's possible concurrent Argon2 working set to roughly 768 MiB. Whether that exhausts a particular runner remains environment-dependent, but the test adds two expensive derivations and can make otherwise-correct tests fail for resource reasons.

  **Suggested change:** Factor the random salt/nonce path so a test can exercise it with the small test KDF parameters, or test the random-header helper directly. Preserve the production `encrypt` path's use of the write defaults while avoiding full-cost key derivation in this randomness-only test.
```

## restated.md

```text
# Correctness

- `F1 / COR-KDF-LIMITS` — **definite** — `src/format_v2.rs:69`, `src/format_v2.rs:153`, `src/format_v2.rs:160`, `src/format_v2.rs:351` — hostile in-range parameters can still force a 4 GiB, 64-pass KDF

  A saltybox2 file stores its Argon2id key-derivation settings in a header supplied by the file itself. Those settings are not authenticated until after the key has been derived, so `decrypt` first checks them against fixed limits and then passes every value within those limits to Argon2. The current limits allow a memory cost of 4,194,304 KiB and a time cost of 64. A crafted payload only needs a complete header and a 16-byte dummy authentication tag to pass the structural checks and make decryption attempt that work; it does not need valid ciphertext or knowledge of the passphrase.

  This defeats the stated purpose of `validate_params` as a resource-exhaustion barrier. The local Argon2 implementation represents the requested memory as 1 KiB blocks and begins `hash_password_into` by allocating a `Vec` containing the full block count, so the maximum accepted memory setting requests about 4 GiB before authentication can reject the file. It then processes that memory for the requested number of passes. Moreover, allocation failure is outside the Argon2 `Result` mapped to `ErrorKind::Argon2Failure`; an out-of-memory allocation can terminate the process rather than produce the handled error. Although this commit does not yet select v2 in the application's format dispatch, `format_v2` and its `decrypt` function are public library APIs, and wiring v2 into normal reads later would expose the same path there.

  Reduce the decrypt-side memory and time maxima to a workload that the supported process can safely accept from untrusted input, and keep that validation before `derive_key`. If files with unusually expensive parameters must remain supported, require an explicit operator-controlled override for them instead of allowing every input file to request 4 GiB and 64 passes.

- `F2 / COR-ENGINE-KDF-COST` — **possible** — `src/format_v2.rs:435`, `src/format_v2.rs:445` — the engine round-trip test adds two unintended full-cost KDF runs

  `test_engine_roundtrip_through_armor` verifies the `V2Engine` trait path by encrypting, removing the textual armor, and decrypting. Its call to `V2Engine.encrypt` delegates to the production `encrypt` function, which derives a key with the 256 MiB write default; decrypting the resulting header derives the key at the same cost again. Immediately afterward, `test_default_params_roundtrip` performs another production-cost encrypt/decrypt pair. That second test is specifically documented as the single test that should exercise the real defaults, while the rest of the module defines `TEST_M`, `TEST_T`, and `TEST_P` to keep tests inexpensive.

  Rust's standard test harness may run these tests at the same time, so the engine test can hold its own 256 MiB Argon2 work buffer while another default-cost test holds another. The exact result depends on runner memory and scheduling, which is why this is a possible rather than definite failure, but the duplicated production-cost derivations add avoidable memory pressure and CPU time and can turn a correct test suite into an out-of-memory failure or timeout on constrained CI and developer machines.

  Keep one production-default round trip, but avoid paying that cost again merely to cover trait dispatch. One option is to make `test_default_params_roundtrip` call the `V2Engine` methods, so it covers both the real defaults and the trait-level armor path. Another is to provide a test-only engine path that uses `TEST_M`, `TEST_T`, and `TEST_P` for `test_engine_roundtrip_through_armor`.

- `F3 / COR-RANDOM-KDF-COST` — **possible** — `src/format_v2.rs:467` — the randomness test performs two more 256 MiB KDF runs

  `test_random_encryption_uses_fresh_salt_and_nonce` is intended to show that two encryptions generate different random salts and nonces. It obtains both headers by calling the production `encrypt` function twice, however, and that function does much more than generate random header fields: each call also runs Argon2id with the 256 MiB production memory setting before sealing the plaintext. The test therefore performs two full-cost key derivations even though the derived keys and ciphertext are irrelevant to its assertion.

  The two calls within this test are sequential, but this test may run concurrently with `test_engine_roundtrip_through_armor` and `test_default_params_roundtrip`. Each of those tests can also have a 256 MiB Argon2 buffer live, putting the module's concurrent Argon2 working set at roughly 768 MiB when all three overlap. Whether that produces an out-of-memory failure or timeout depends on the machine, but it adds substantial resource-driven flakiness to a test of random header generation.

  Separate random salt and nonce generation from the production-cost derivation sufficiently for this test to exercise the random-header path with `TEST_M`, `TEST_T`, and `TEST_P`, or test a factored random-header helper directly. The production `encrypt` function should continue using the real write defaults; only this randomness-focused test needs the cheaper route.

Restated: 3/3 findings
```

## REPORT.md

```text
# Pre-PR Review Swarm Report

Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 1/1 reviewers completed; the panel was restricted to correctness by the user.

Reviewer continuation: 1/1 second passes, 0/0 third passes; unavailable: none; capped with new findings: none

Restatement: 3/3 findings restated, 1 attempt

Finding accounting: 3 reviewer findings → 3 reported (0 same-location merges, 0 rejected)

# Correctness

- `F1 / COR-KDF-LIMITS` — **definite** — `src/format_v2.rs:69`, `src/format_v2.rs:153`, `src/format_v2.rs:160`, `src/format_v2.rs:351` — hostile in-range parameters can still force a 4 GiB, 64-pass KDF

  A saltybox2 file stores its Argon2id key-derivation settings in a header supplied by the file itself. Those settings are not authenticated until after the key has been derived, so `decrypt` first checks them against fixed limits and then passes every value within those limits to Argon2. The current limits allow a memory cost of 4,194,304 KiB and a time cost of 64. A crafted payload only needs a complete header and a 16-byte dummy authentication tag to pass the structural checks and make decryption attempt that work; it does not need valid ciphertext or knowledge of the passphrase.

  This defeats the stated purpose of `validate_params` as a resource-exhaustion barrier. The local Argon2 implementation represents the requested memory as 1 KiB blocks and begins `hash_password_into` by allocating a `Vec` containing the full block count, so the maximum accepted memory setting requests about 4 GiB before authentication can reject the file. It then processes that memory for the requested number of passes. Moreover, allocation failure is outside the Argon2 `Result` mapped to `ErrorKind::Argon2Failure`; an out-of-memory allocation can terminate the process rather than produce the handled error. Although this commit does not yet select v2 in the application's format dispatch, `format_v2` and its `decrypt` function are public library APIs, and wiring v2 into normal reads later would expose the same path there.

  Reduce the decrypt-side memory and time maxima to a workload that the supported process can safely accept from untrusted input, and keep that validation before `derive_key`. If files with unusually expensive parameters must remain supported, require an explicit operator-controlled override for them instead of allowing every input file to request 4 GiB and 64 passes.

- `F2 / COR-ENGINE-KDF-COST` — **possible** — `src/format_v2.rs:435`, `src/format_v2.rs:445` — the engine round-trip test adds two unintended full-cost KDF runs

  `test_engine_roundtrip_through_armor` verifies the `V2Engine` trait path by encrypting, removing the textual armor, and decrypting. Its call to `V2Engine.encrypt` delegates to the production `encrypt` function, which derives a key with the 256 MiB write default; decrypting the resulting header derives the key at the same cost again. Immediately afterward, `test_default_params_roundtrip` performs another production-cost encrypt/decrypt pair. That second test is specifically documented as the single test that should exercise the real defaults, while the rest of the module defines `TEST_M`, `TEST_T`, and `TEST_P` to keep tests inexpensive.

  Rust's standard test harness may run these tests at the same time, so the engine test can hold its own 256 MiB Argon2 work buffer while another default-cost test holds another. The exact result depends on runner memory and scheduling, which is why this is a possible rather than definite failure, but the duplicated production-cost derivations add avoidable memory pressure and CPU time and can turn a correct test suite into an out-of-memory failure or timeout on constrained CI and developer machines.

  Keep one production-default round trip, but avoid paying that cost again merely to cover trait dispatch. One option is to make `test_default_params_roundtrip` call the `V2Engine` methods, so it covers both the real defaults and the trait-level armor path. Another is to provide a test-only engine path that uses `TEST_M`, `TEST_T`, and `TEST_P` for `test_engine_roundtrip_through_armor`.

- `F3 / COR-RANDOM-KDF-COST` — **possible** — `src/format_v2.rs:467` — the randomness test performs two more 256 MiB KDF runs

  `test_random_encryption_uses_fresh_salt_and_nonce` is intended to show that two encryptions generate different random salts and nonces. It obtains both headers by calling the production `encrypt` function twice, however, and that function does much more than generate random header fields: each call also runs Argon2id with the 256 MiB production memory setting before sealing the plaintext. The test therefore performs two full-cost key derivations even though the derived keys and ciphertext are irrelevant to its assertion.

  The two calls within this test are sequential, but this test may run concurrently with `test_engine_roundtrip_through_armor` and `test_default_params_roundtrip`. Each of those tests can also have a 256 MiB Argon2 buffer live, putting the module's concurrent Argon2 working set at roughly 768 MiB when all three overlap. Whether that produces an out-of-memory failure or timeout depends on the machine, but it adds substantial resource-driven flakiness to a test of random header generation.

  Separate random salt and nonce generation from the production-cost derivation sufficiently for this test to exercise the random-header path with `TEST_M`, `TEST_T`, and `TEST_P`, or test a factored random-header helper directly. The production `encrypt` function should continue using the real write defaults; only this randomness-focused test needs the cheaper route.

Swarm run: 20260905-0442-62d866d-4984-combined-sol-high

Run log: RUN_LOG_DIR/20260905-0442-62d866d-4984-combined-sol-high.md
```

## BUCKETS.md

```text
# Nofix confirmation and buckets

No source changes were applied. These are the decisions the default fixing mode would have made after checking each premise against the reviewed checkout.

### F1 / COR-KDF-LIMITS

- Origin: `correctness-combined p1`
- Confidence: **definite**, unchanged after confirmation
- Bucket: **would surface**

The claim is confirmed. `decrypt` accepts header-selected costs through 4,194,304 KiB and 64 passes, validates them before authentication, and calls Argon2 0.5.3, whose allocating API creates one 1 KiB block per configured memory unit. A structurally complete payload can therefore request roughly 4 GiB of work memory before authentication rejects it.

The fix requires choosing a safe resource policy. Lowering the maxima changes which existing on-disk inputs the public v2 API accepts, while an operator override adds API or configuration behavior. Those are explicit bucket-2 tradeoffs rather than an unattended constant edit. The user decision is which default ceiling the supported process should guarantee and whether compatibility above it needs an opt-in override. The reviewer's larger override proposal was therefore not treated as an automatic repair.

### F2 / COR-ENGINE-KDF-COST

- Origin: `correctness-combined p1`
- Confidence: **possible**, unchanged after confirmation
- Bucket: **would fix**

The premise is confirmed: the engine test and the documented production-default test each perform a 256 MiB encrypt/decrypt pair, and the standard test harness may overlap them. The actual failure remains machine- and scheduling-dependent, so the confidence stays possible.

The smallest complete repair is local and non-structural: route the existing production-default test through `V2Engine.encrypt` and `V2Engine.decrypt`, then remove the redundant engine round-trip test. This keeps one real-default round trip, preserves trait-path coverage, adds no symbol, and fits within the twenty-line eligibility limit.

### F3 / COR-RANDOM-KDF-COST

- Origin: `correctness-combined p1`
- Confidence: **possible**, unchanged after confirmation
- Bucket: **would fix**

The premise is confirmed: the randomness test calls production `encrypt` twice, and each call derives with the 256 MiB default even though the assertions inspect only the salt and nonce. Concurrent resource failure remains dependent on the runner, so the confidence stays possible.

The repair can remain inside the existing test: exercise the existing random-byte helper for fresh salt and nonce values while the deterministic and default-round-trip tests continue to cover header construction and production encryption. This removes both full-cost derivations without adding production structure and stays within the local, non-structural limit.
```

## RUN-LOG.md

```text
# 20260905-0442-62d866d-4984-combined-sol-high

- Timestamp: `2026-09-05T05:01:22Z`
- Repository root: `EVAL_ROOT/combined-sol-high/repo`
- Commit: `62d866d6a57a24ef6bb329b28a246b44b758ff7a`
- Scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)
- Skill arguments: `nofix commit`
- Coordinator: `gpt-5.6-sol` at `high` effort
- Panel restriction: correctness only, as configured by the user

## Findings report

Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 1/1 reviewers completed; the panel was restricted to correctness by the user.

Reviewer continuation: 1/1 second passes, 0/0 third passes; unavailable: none; capped with new findings: none

Restatement: 3/3 findings restated, 1 attempt

Finding accounting: 3 reviewer findings → 3 reported (0 same-location merges, 0 rejected)

### Correctness

- `F1 / COR-KDF-LIMITS` — **definite** — `src/format_v2.rs:69`, `src/format_v2.rs:153`, `src/format_v2.rs:160`, `src/format_v2.rs:351` — hostile in-range parameters can still force a 4 GiB, 64-pass KDF

  A saltybox2 file stores its Argon2id key-derivation settings in a header supplied by the file itself. Those settings are not authenticated until after the key has been derived, so `decrypt` first checks them against fixed limits and then passes every value within those limits to Argon2. The current limits allow a memory cost of 4,194,304 KiB and a time cost of 64. A crafted payload only needs a complete header and a 16-byte dummy authentication tag to pass the structural checks and make decryption attempt that work; it does not need valid ciphertext or knowledge of the passphrase.

  This defeats the stated purpose of `validate_params` as a resource-exhaustion barrier. The local Argon2 implementation represents the requested memory as 1 KiB blocks and begins `hash_password_into` by allocating a `Vec` containing the full block count, so the maximum accepted memory setting requests about 4 GiB before authentication can reject the file. It then processes that memory for the requested number of passes. Moreover, allocation failure is outside the Argon2 `Result` mapped to `ErrorKind::Argon2Failure`; an out-of-memory allocation can terminate the process rather than produce the handled error. Although this commit does not yet select v2 in the application's format dispatch, `format_v2` and its `decrypt` function are public library APIs, and wiring v2 into normal reads later would expose the same path there.

  Reduce the decrypt-side memory and time maxima to a workload that the supported process can safely accept from untrusted input, and keep that validation before `derive_key`. If files with unusually expensive parameters must remain supported, require an explicit operator-controlled override for them instead of allowing every input file to request 4 GiB and 64 passes.

- `F2 / COR-ENGINE-KDF-COST` — **possible** — `src/format_v2.rs:435`, `src/format_v2.rs:445` — the engine round-trip test adds two unintended full-cost KDF runs

  `test_engine_roundtrip_through_armor` verifies the `V2Engine` trait path by encrypting, removing the textual armor, and decrypting. Its call to `V2Engine.encrypt` delegates to the production `encrypt` function, which derives a key with the 256 MiB write default; decrypting the resulting header derives the key at the same cost again. Immediately afterward, `test_default_params_roundtrip` performs another production-cost encrypt/decrypt pair. That second test is specifically documented as the single test that should exercise the real defaults, while the rest of the module defines `TEST_M`, `TEST_T`, and `TEST_P` to keep tests inexpensive.

  Rust's standard test harness may run these tests at the same time, so the engine test can hold its own 256 MiB Argon2 work buffer while another default-cost test holds another. The exact result depends on runner memory and scheduling, which is why this is a possible rather than definite failure, but the duplicated production-cost derivations add avoidable memory pressure and CPU time and can turn a correct test suite into an out-of-memory failure or timeout on constrained CI and developer machines.

  Keep one production-default round trip, but avoid paying that cost again merely to cover trait dispatch. One option is to make `test_default_params_roundtrip` call the `V2Engine` methods, so it covers both the real defaults and the trait-level armor path. Another is to provide a test-only engine path that uses `TEST_M`, `TEST_T`, and `TEST_P` for `test_engine_roundtrip_through_armor`.

- `F3 / COR-RANDOM-KDF-COST` — **possible** — `src/format_v2.rs:467` — the randomness test performs two more 256 MiB KDF runs

  `test_random_encryption_uses_fresh_salt_and_nonce` is intended to show that two encryptions generate different random salts and nonces. It obtains both headers by calling the production `encrypt` function twice, however, and that function does much more than generate random header fields: each call also runs Argon2id with the 256 MiB production memory setting before sealing the plaintext. The test therefore performs two full-cost key derivations even though the derived keys and ciphertext are irrelevant to its assertion.

  The two calls within this test are sequential, but this test may run concurrently with `test_engine_roundtrip_through_armor` and `test_default_params_roundtrip`. Each of those tests can also have a 256 MiB Argon2 buffer live, putting the module's concurrent Argon2 working set at roughly 768 MiB when all three overlap. Whether that produces an out-of-memory failure or timeout depends on the machine, but it adds substantial resource-driven flakiness to a test of random header generation.

  Separate random salt and nonce generation from the production-cost derivation sufficiently for this test to exercise the random-header path with `TEST_M`, `TEST_T`, and `TEST_P`, or test a factored random-header helper directly. The production `encrypt` function should continue using the real write defaults; only this randomness-focused test needs the cheaper route.

## Nofix decisions

### F1 / COR-KDF-LIMITS

- Reviewer category and origin: correctness, `correctness-combined p1`
- Confidence: **definite** as filed; unchanged after confirmation
- Bucket: **would surface**

The claim is confirmed. `decrypt` accepts header-selected costs through 4,194,304 KiB and 64 passes, validates them before authentication, and calls Argon2 0.5.3, whose allocating API creates one 1 KiB block per configured memory unit. A structurally complete payload can therefore request roughly 4 GiB of work memory before authentication rejects it. The fix requires choosing a safe resource policy: lowering the maxima changes accepted on-disk inputs, while an override adds API or configuration behavior. The user must choose the guaranteed ceiling and whether higher-cost compatibility is opt-in. The reviewer's larger override proposal was not applied.

### F2 / COR-ENGINE-KDF-COST

- Reviewer category and origin: correctness, `correctness-combined p1`
- Confidence: **possible** as filed; unchanged after confirmation because failure depends on runner memory and scheduling
- Bucket: **would fix**

The duplicate production-cost round trip is confirmed. The smallest complete fix is local and non-structural: use `V2Engine.encrypt` and `V2Engine.decrypt` in the existing production-default test, then delete the redundant engine round-trip test. This keeps one real-default round trip and trait-path coverage without adding a symbol or exceeding the twenty-line limit. No larger proposal would be applied.

### F3 / COR-RANDOM-KDF-COST

- Reviewer category and origin: correctness, `correctness-combined p1`
- Confidence: **possible** as filed; unchanged after confirmation because failure depends on runner memory and scheduling
- Bucket: **would fix**

The two unnecessary full-cost derivations are confirmed. The repair can stay inside the existing test by exercising the existing random-byte helper for fresh salt and nonce values, while the deterministic and default-round-trip tests continue to cover header construction and production encryption. This is local, non-structural, and within the twenty-line limit. No larger proposal would be applied.
```
