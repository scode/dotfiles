# baseline: original review outputs

These are the recorded outputs, including incorrect or disputed claims. The parent assessment is in
[FINDINGS.md](FINDINGS.md). Local paths and native identifiers are replaced; finding wording is retained. Source paths
refer to the pinned saltybox commit.

## correctness-data-flow-p1.md

```text
- **possible — `src/format_v2.rs:69-81, 160-167` — accepted KDF limits still allow a resource-exhaustion payload**

  What happens: The v2 decrypt path accepts attacker-controlled Argon2 parameters up to `m = 4,194,304` KiB and `t = 64`, then passes them to `derive_key` before it can authenticate the payload. A structurally valid payload can therefore make Argon2 allocate about 4 GiB and perform 64 passes using only a 52-byte header plus an authentication tag. On hosts without that memory the process can abort during allocation; on hosts that have it, repeated inputs can consume substantial CPU and memory. The module's stated purpose for these caps is to prevent a hostile file from becoming a memory or CPU bomb, but these maxima still permit one.

  Why it matters: Decrypting an untrusted file can become a denial of service instead of returning an error, and the failure happens before the passphrase or ciphertext is checked.

  Suggested change: Set a resource budget appropriate for the supported hosts (or make it an explicit caller-configured budget) and reject headers exceeding that budget before `derive_key`; cover the chosen boundary with a test that never invokes the KDF for over-budget input.
```

## correctness-data-flow-p2.md

```text
[]

Completed the additional correctness data-flow pass over the stated scope and found nothing new.
```

## correctness-edge-inputs-p1.md

```text
- **definite — saltybox2 is never reachable through format dispatch**

  What happens: `src/lib.rs:8` exposes `format_v2::V2Engine`, but the dispatcher in `src/format.rs:72-78` still defines `ENGINES` with only `V1Engine` and keeps `V1Engine` as `default_write_engine()`. Consequently, `format::engine_for` rejects every `saltybox2:` input as an unsupported future version, and all normal writes continue to produce saltybox1 files. The new golden tests exercise `V2Engine` directly, so they do not cover the public dispatch/CLI path.

  Why it matters: The commit adds a complete saltybox2 implementation that users of the format abstraction cannot actually read or select for writing. A saltybox2 file is therefore unusable through the normal format selection path, and the advertised versioned format behavior is not delivered.

  Suggested change: Register a `V2Engine` instance in `format::ENGINES` and make it the intended `default_write_engine()` when this feature is meant to become the write format; add dispatch coverage for selecting and decrypting a saltybox2 armored input and for the chosen default.

- **definite — the advertised parameter caps still permit a resource-exhaustion payload**

  What happens: `src/format_v2.rs:69-81` accepts `m_cost_kib` up to `4_194_304` (4 GiB of Argon2 memory) and `t_cost` up to 64. `decrypt` validates only those fixed maxima and then calls `derive_key` at `src/format_v2.rs:351-352`, which allocates the claimed memory and performs the claimed passes before authentication. Any caller that accepts an untrusted saltybox2 payload can submit a header at those limits with arbitrary sealed bytes and force this work.

  Why it matters: On ordinary hosts this can exhaust available memory or trigger severe swapping, and the maximum combination also permits an extreme CPU burn before the authentication check can reject the payload. That contradicts the stated purpose of the caps (preventing a hostile file from becoming a memory or CPU bomb) and lets unauthenticated input control a multi-gigabyte allocation.

  Suggested change: Set conservative memory and time limits appropriate to the supported runtime (and reject values that exceed an explicit resource budget before KDF), or otherwise make the resource budget an enforced caller-controlled limit rather than treating 4 GiB and 64 passes as safe universal bounds.
```

## correctness-edge-inputs-p2.md

```text
[]

Completed the additional pass over the stated scope and found nothing new.
```

## correctness-general-p1.md

```text
- **definite** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:342-352`

  **What happens:** A saltybox2 header controls the Argon2 memory and time costs. `validate_params` accepts each value independently up to 4 GiB of memory and 64 passes, then `decrypt` runs Argon2 before authenticating the file. An attacker can therefore supply the fully valid combination `m=4194304, t=64, p=8` and make one decrypt attempt allocate about 4 GiB and process that memory 64 times.

  **Why it matters:** The validation is described as the barrier that prevents hostile files from becoming memory or CPU bombs, but the largest accepted combination is itself a practical denial of service. It can exhaust the process or host memory, or keep the decrypting process busy for a very long time, before the authentication failure proves the input was bogus.

  **Suggested change:** Set decrypt-side limits to a resource budget the supported machines can actually tolerate, and validate the combined work as well as the individual fields (for example, cap `m_cost_kib * t_cost` with checked arithmetic). If unusually expensive historical files must remain readable, require an explicit opt-in for costs above the safe automatic-decrypt budget rather than accepting them from every unauthenticated header.
```

## correctness-general-p2.md

```text
- **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213`

  **What happens:** The new `argon2` dependency is enabled with its default features only. Its optional `zeroize` feature is not enabled, so the large Argon2 work-memory allocation is released without being scrubbed after `hash_password_into` returns. Wrapping the 32-byte output key in `Zeroizing` clears that output, but it does not clear the password-derived Argon2 blocks from which the key was computed.

  **Why it matters:** On the write defaults this leaves about 256 MiB of password-derived state in freed heap pages after every encryption or decryption. A later memory disclosure, core dump, or allocator reuse can expose enough of that state to undermine the intended wiping of the passphrase and derived key. The practical exploitability depends on what other process-memory access is available, which is why this is tagged possible.

  **Suggested change:** Enable the Argon2 crate's `zeroize` feature in `Cargo.toml` so its work-memory blocks are wiped before deallocation, and keep the existing `Zeroizing` wrapper around the derived output key.
```

## correctness-general-p3.md

```text
- **definite** — `src/format_v2.rs:193-211`

  **What happens:** Both fallible Argon2 operations in `derive_key` convert the underlying `argon2::Error` to display text and then construct `SaltyboxError` with `with_kind`, which has no source. The caller receives the `Argon2Failure` tag and a rendered message, but `std::error::Error::source` cannot reach the error returned by `Params::new` or `hash_password_into`.

  **Why it matters:** This silently breaks the crate's structured error chain on the new KDF path. Library callers cannot inspect the originating error, and the CLI's source-chain reporter cannot report it as a cause after higher-level context is added. Keeping a copy of `Display` output in the wrapper message is not equivalent to propagating the original error because it discards its type and any future source-chain detail.

  **Suggested change:** Preserve each Argon2 error with `SaltyboxError::with_kind_and_source`, enabling the Argon2 crate's `std` support if that is required for its error type to implement `std::error::Error`. Keep the stable high-level message and `ErrorKind::Argon2Failure` on the wrapper.
```

## correctness-state-lifecycle-p1.md

```text
- **[definite] The advertised KDF limits still let a tiny input force a 4 GiB, 64-pass job** (`src/format_v2.rs:69`, `src/format_v2.rs:351`)

  **What happens:** A saltybox2 payload carries its own Argon2 memory and time costs. `validate_params` accepts `m_cost_kib = 4_194_304` and `t_cost = 64`, then `decrypt` runs Argon2 before the authentication tag can be checked. Consequently, a structurally valid 68-byte payload can force an attempted 4 GiB allocation followed by as many as 64 passes over that memory, even though it contains only a dummy tag and can never authenticate.

  **Why it matters:** The comments describe these limits as the barrier against hostile-input memory and CPU bombs, but an unauthenticated file at the accepted maxima can still exhaust memory, trigger allocator/OOM termination, or occupy the process for a prolonged period. This will become reachable from ordinary file decryption when the already-public engine is registered.

  **Suggested change:** Set unauthenticated-work limits that are safe for the supported machines before freezing the format, or require an explicit opt-in for parameter sets above the normal operational budget. Also allocate the Argon2 workspace fallibly so an unsatisfied request returns a `SaltyboxError` instead of terminating the process.

- **[definite] Argon2's passphrase-derived workspace is freed without being wiped** (`src/format_v2.rs:182`, `Cargo.toml:18`)

  **What happens:** `derive_key` wraps only the final 32-byte key in `Zeroizing` and calls `argon2::Argon2::hash_password_into`. With the dependency features selected here, that API internally allocates the Argon2 block workspace as a normal `Vec` and drops it without zeroing it. The workspace is 256 MiB for every normal v2 operation and contains passphrase-derived state, including the final lane blocks from which the key is produced.

  **Why it matters:** Successful, failed, and repeated operations leave sensitive KDF state in freed heap pages even though the surrounding code deliberately wipes passphrases, derived keys, and decrypted plaintext. A core dump, process-memory disclosure, or later allocator misuse can expose material that can be used to reconstruct the derived key.

  **Suggested change:** Enable Argon2's `zeroize` support, allocate its `Block` workspace explicitly, wrap that allocation in `Zeroizing`, and call `hash_password_into_with_memory` so cleanup runs on both success and error paths. Verify that the dependency's other password-derived temporaries are also covered.

- **[possible] Three parallel unit tests perform six write-default Argon2 derivations** (`src/format_v2.rs:436`, `src/format_v2.rs:448`, `src/format_v2.rs:468`)

  **What happens:** The test constants say that only `test_default_params_roundtrip` exercises the 256 MiB write defaults, but `test_engine_roundtrip_through_armor` also performs a default-cost encrypt and decrypt, while `test_random_encryption_uses_fresh_salt_and_nonce` performs two more default-cost encryptions. The Rust test harness can run these three tests concurrently, producing up to three simultaneous 256 MiB workspaces and six expensive derivations in total.

  **Why it matters:** On a memory-constrained or highly parallel test runner, this can cause avoidable swapping, OOM termination, or a suite duration well beyond the stated goal of keeping all but one round-trip cheap. The exact failure depends on runner resources, so the operational consequence remains possible rather than certain.

  **Suggested change:** Keep a single test that performs the write-default round-trip and parameter assertions. Exercise engine delegation and random salt/nonce generation through a test-only helper that accepts the small KDF parameters, so those tests preserve their behavioral coverage without allocating the production workspace.
```

## correctness-state-lifecycle-p2.md

```text
[]

Completed the additional pass over current commit 62d866d (9 touched files, 958 diff lines; correctness only) and found nothing new.
```

## correctness-systems-p1.md

```text
- **possible — An unauthenticated header can request a 4 GiB, 64-pass KDF** (`src/format_v2.rs:69-81`, `src/format_v2.rs:351-352`)

  **What happens:** A saltybox2 header is attacker-controlled until AEAD authentication finishes, but `validate_params` accepts `m_cost_kib = 4,194,304` and `t_cost = 64`. `decrypt` then feeds those values to Argon2 before checking the authentication tag. A payload only 68 bytes long can therefore make the process attempt a 4 GiB allocation and, if that succeeds, perform up to 64 passes over it. The Argon2 API used here allocates with `vec!`, so allocation failure can abort the process instead of returning the declared `Result` error.

  **Why it matters:** Whether the process is killed or becomes unusably slow depends on the host's memory and allocator, but the accepted values provide a large resource-amplification attack despite the surrounding comment saying the caps prevent hostile files from becoming memory or CPU bombs. Once this engine is put on the normal read path, merely attempting to decrypt an untrusted tiny file can terminate or stall saltybox.

  **Suggested change:** Set the automatic decrypt limits to costs that the minimum supported host can safely run, ideally close to the largest parameters saltybox actually writes. If compatibility requires larger historical parameters, require an explicit user opt-in for them. Also allocate the Argon2 workspace with a fallible reservation so an unsupported allocation is returned as a structured error rather than reaching `vec!`'s OOM path.

- **definite — Argon2's password-derived workspace is freed without being erased** (`Cargo.toml:18`, `src/format_v2.rs:202-211`)

  **What happens:** `derive_key` wraps only the final 32-byte key in `Zeroizing` and calls `Argon2::hash_password_into`. With the dependency declaration in this change, Argon2's optional `zeroize` support is disabled; this API allocates its memory-hard workspace as an ordinary `Vec<Block>` and drops it without clearing it. That workspace includes the final lane blocks from which the returned encryption key is computed, so wiping the 32-byte output still leaves password-derived key material in freed heap memory.

  **Why it matters:** The new engine deliberately wipes the passphrase-facing key and plaintext buffers, but the much larger KDF buffer survives until overwritten by later allocations. A crash dump, post-operation heap disclosure, or memory-reuse bug can therefore recover enough Argon2 state to reconstruct the file key after the nominal key buffer has been erased.

  **Suggested change:** Enable Argon2's `zeroize` feature, allocate the block workspace in this crate, wrap that allocation in `Zeroizing` (or otherwise erase it on every success and error path), and call `hash_password_into_with_memory` so the password-derived blocks are cleared before deallocation.
```

## correctness-systems-p2.md

```text
- **definite — Three parallel unit tests run the 256 MiB production KDF instead of the intended single coverage case** (`src/format_v2.rs:379-381`, `src/format_v2.rs:434-474`)

  **What happens:** The test module says the production Argon2 defaults are exercised once by `test_default_params_roundtrip`, with other tests using the 8 MiB `TEST_M` setting. However, `test_engine_roundtrip_through_armor` also calls `V2Engine.encrypt`, and `test_random_encryption_uses_fresh_salt_and_nonce` calls `encrypt` twice. Those paths all use `DEFAULT_M_COST_KIB = 262,144`, and the engine round-trip and explicit default test each derive at that cost again while decrypting. A normal unit-test run therefore performs six 256 MiB Argon2 derivations across three tests; the Rust test harness may run the three tests concurrently, producing at least 768 MiB of simultaneous KDF workspace before accounting for the small-parameter tests.

  **Why it matters:** The test suite consumes several times the memory and CPU its own test design claims, and its peak depends on test-thread scheduling. This can make otherwise-correct builds time out or be killed on memory-constrained CI and developer machines, while duplicating coverage already supplied by the one deliberate production-default round-trip.

  **Suggested change:** Keep only `test_default_params_roundtrip` on the production defaults. Give the engine round-trip and randomness tests a test-only random-encryption helper that still generates fresh salt and nonce but accepts `TEST_M`, `TEST_T`, and `TEST_P`, or otherwise arrange those tests to exercise their intended behavior with the small parameters.
```

## correctness-systems-p3.md

```text
[]

Completed the final pass over the stated scope and found nothing new.
```

## merged.md

```text
# Merged correctness findings

- `F1 / COR-DISPATCH` — **definite** — `src/lib.rs:8`, `src/format.rs:72-78` — saltybox2 is unavailable through the normal format dispatcher

**Provenance:** correctness-edge-inputs p1

**What happens:** `src/lib.rs` publicly exposes `format_v2::V2Engine`, but `format::ENGINES` still contains only `V1Engine`, and `default_write_engine()` also returns `V1Engine`. The normal dispatcher therefore rejects every `saltybox2:` input as an unsupported future version, while ordinary writes continue to emit saltybox1. The new golden tests call `V2Engine` directly and do not exercise the public dispatch or CLI path.

**Why it matters:** The commit adds a complete saltybox2 implementation that callers of the format abstraction cannot read or select for writing. A saltybox2 file is unusable through the normal application path, so the new format is present in source but unavailable to users.

**Suggested change:** Register `V2Engine` in `format::ENGINES`, select it as the write default if this commit is intended to activate saltybox2, and cover public dispatch for reading saltybox2 plus the intended write default.

- `F2 / COR-KDF-BUDGET` — **possible** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352` — unauthenticated input can request an extreme Argon2 workload

**Provenance:** correctness-general p1; correctness-data-flow p1; correctness-state-lifecycle p1; correctness-systems p1; correctness-edge-inputs p1. Five correctness lenses agreed. The merged confidence is the lower input tag.

**What happens:** A saltybox2 header controls the Argon2 memory and time costs. `validate_params` accepts each value independently up to 4 GiB of memory, 64 passes, and eight lanes, then decryption runs Argon2 before the file can be authenticated. A tiny structurally valid payload can therefore request the fully accepted maximum and make one decrypt attempt allocate about 4 GiB and process that memory up to 64 times. On a host that cannot satisfy the allocation, the Argon2 path can reach an infallible `Vec` allocation rather than return the declared error.

**Why it matters:** The validation is documented as the boundary that keeps hostile files from becoming memory or CPU bombs, but the maximum accepted combination can exhaust memory, trigger severe swapping or termination, or keep the process busy for a long time before authentication rejects the input. The exact operational effect depends on the host, so the merged finding remains possible.

**Suggested change:** Enforce an unauthenticated-decrypt resource budget that supported hosts can safely run, including the combined work rather than only independent field maxima. Require an explicit opt-in for unusually expensive parameter sets if compatibility needs them, and make workspace allocation failure return a structured error.

- `F3 / COR-KDF-ZEROIZE` — **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213` — Argon2's passphrase-derived workspace is freed without being erased

**Provenance:** correctness-systems p1; correctness-state-lifecycle p1; correctness-general p2. Three correctness lenses agreed. The merged confidence is the lower input tag.

**What happens:** `derive_key` wraps the final 32-byte key in `Zeroizing`, but the new `argon2` dependency does not enable its optional `zeroize` support. `hash_password_into` allocates the much larger memory-hard workspace as ordinary blocks and releases them without clearing them. At the default write cost, about 256 MiB of passphrase-derived state can remain in freed heap pages after encryption or decryption.

**Why it matters:** The new engine deliberately wipes the passphrase, final key, and plaintext buffers, but a crash dump, later process-memory disclosure, or allocator reuse could expose the Argon2 state left in freed memory. The practical recovery risk depends on what memory access an attacker has, so the merged finding remains possible.

**Suggested change:** Enable the Argon2 crate's zeroization support, explicitly allocate its block workspace, wrap that workspace in `Zeroizing`, and use the API that accepts caller-provided memory so the password-derived blocks are erased on success and error paths.

- `F4 / COR-ERROR-SOURCE` — **definite** — `src/format_v2.rs:193-211` — Argon2 failures lose their structured source error

**Provenance:** correctness-general p3

**What happens:** Both fallible Argon2 operations in `derive_key` turn `argon2::Error` into display text and then construct `SaltyboxError` with `with_kind`, which has no source. The caller gets the `Argon2Failure` tag and rendered message, but `std::error::Error::source` cannot reach the original error from `Params::new` or `hash_password_into`.

**Why it matters:** The new KDF path breaks the crate's structured error chain. Library callers cannot inspect the originating error, and the CLI's source-chain reporter cannot show it as a cause after higher-level context is added. Copying the display text does not preserve the error type or any future source detail.

**Suggested change:** Preserve each Argon2 error with `SaltyboxError::with_kind_and_source`, enabling Argon2's `std` support if its error type needs that feature to implement `std::error::Error`. Keep the stable wrapper message and `ErrorKind::Argon2Failure`.

- `F5 / COR-TEST-KDF-COST` — **possible** — `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474` — three parallel tests run the 256 MiB production KDF

**Provenance:** correctness-state-lifecycle p1; correctness-systems p2. Two correctness lenses agreed. The merged confidence is the lower input tag.

**What happens:** The test module says only `test_default_params_roundtrip` exercises the 256 MiB production Argon2 defaults, while other tests should use the 8 MiB test setting. `test_engine_roundtrip_through_armor` also performs a default-cost encrypt and decrypt, and `test_random_encryption_uses_fresh_salt_and_nonce` performs two more default-cost encryptions. A normal unit-test run therefore performs six 256 MiB derivations across three tests, which the Rust test harness may run concurrently.

**Why it matters:** Parallel execution can create at least 768 MiB of simultaneous KDF workspace and duplicate expensive work that one deliberate production-default round trip already covers. Depending on the runner, this can cause avoidable swapping, out-of-memory termination, or long test times.

**Suggested change:** Keep only `test_default_params_roundtrip` on production defaults. Give the engine round-trip and randomness tests a test-only path that still exercises fresh salt and nonce generation while accepting the small KDF parameters.
```

## restated-attempt-1.md

```text
# Merged correctness findings

## F1 / COR-DISPATCH

**Confidence:** definite

**Anchors:** `src/lib.rs:8`, `src/format.rs:72-78`

**Provenance:** correctness-edge-inputs p1

The crate now publicly exposes a `format_v2` module and its `V2Engine`, which can directly encrypt and decrypt the new `saltybox2:` representation. The application-facing format layer is still wired exclusively to version 1, however: its `ENGINES` table contains only `V1Engine`, and `default_write_engine()` also returns `V1Engine`. Both CLI decryption and update operations call `format::decode`, so they classify a valid `saltybox2:` file as an unsupported future version without ever asking `V2Engine` to decode it. Encryption and the rewrite phase of an update likewise keep producing `saltybox1:` files.

This leaves version 2 reachable only by library code that deliberately bypasses the normal dispatcher. The golden-vector test does exactly that by invoking `V2Engine` directly, so it demonstrates the engine's implementation without catching the missing application integration. Register `V2Engine` in `format::ENGINES`, and, if this commit is meant to activate version 2 for new files, return it from `default_write_engine()`. Add coverage that reads a `saltybox2:` value through `format::decode` and asserts whichever engine is intended to be the write default.

## F2 / COR-KDF-BUDGET

**Confidence:** possible

**Anchors:** `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352`

**Provenance:** correctness-general p1; correctness-data-flow p1; correctness-state-lifecycle p1; correctness-systems p1; correctness-edge-inputs p1. Five correctness lenses agreed. The merged confidence is the lower input tag.

A `saltybox2:` file stores its Argon2 memory, pass-count, and lane settings in its unauthenticated header. Decryption parses those values and validates each against a separate ceiling before it can authenticate the ciphertext. The accepted ceilings allow one small input to request 4,194,304 KiB of memory, 64 passes, and eight lanes together. Argon2 then allocates one 1 KiB block per requested KiB and performs the requested work before XChaCha20-Poly1305 can reject a bad passphrase or forged file, so an attacker-controlled file can make a single decrypt attempt reserve about 4 GiB and repeatedly process it.

That maximum may cause heavy swapping, a very long operation, or process termination on a supported machine with less available memory. The dependency's convenience API creates its workspace with an ordinary `vec!` allocation, so allocation exhaustion follows Rust's infallible allocation path rather than becoming this crate's structured `SaltyboxError`. The exact outcome is host-dependent, which is why this remains a possible finding. Set a combined memory-and-work budget appropriate for supported hosts instead of relying only on independent field maxima, require explicit opt-in when compatibility demands unusually expensive parameters, and allocate caller-provided Argon2 workspace through a fallible path so allocation failure can be returned as a structured error.

## F3 / COR-KDF-ZEROIZE

**Confidence:** possible

**Anchors:** `Cargo.toml:18`, `src/format_v2.rs:182-213`

**Provenance:** correctness-systems p1; correctness-state-lifecycle p1; correctness-general p2. Three correctness lenses agreed. The merged confidence is the lower input tag.

`derive_key` protects its 32-byte output key with `Zeroizing`, but `argon2 = "0.5"` uses the Argon2 crate's default features and does not enable its optional `zeroize` feature. `hash_password_into` internally allocates a vector of 1 KiB blocks for the memory-hard computation; at this format's default memory setting, that workspace is about 256 MiB. Those blocks contain passphrase-derived intermediate state, and the vector is deallocated without first clearing its contents.

The code therefore wipes the passphrase buffer, final key, and decrypted plaintext while leaving the largest passphrase-derived buffer behind in freed allocator pages. Whether that state can be recovered depends on a later memory disclosure, crash dump, or allocator reuse, so the practical impact remains possible. Enable Argon2's `zeroize` feature so its block type can be erased, allocate the workspace explicitly in a `Zeroizing` container, and call `hash_password_into_with_memory`; that makes the workspace cleanup apply on both success and error paths rather than relying on the dependency's internally allocated ordinary vector.

## F4 / COR-ERROR-SOURCE

**Confidence:** definite

**Anchors:** `src/format_v2.rs:193-211`

**Provenance:** correctness-general p3

`derive_key` handles failures from both `Params::new` and `hash_password_into` by interpolating `argon2::Error` into a string and constructing `SaltyboxError` with `with_kind`. That constructor deliberately stores no source error. The resulting value retains the `Argon2Failure` kind and readable text, but the original typed Argon2 error is no longer present in the `std::error::Error` source chain.

This matters at both library and CLI boundaries. A library caller cannot inspect the underlying cause, and when `file_ops` adds its higher-level decryption or encryption context, the CLI's source-chain loop reaches the source-less Argon2 wrapper and stops. Preserve the original error with `with_kind_and_source` while keeping the stable wrapper message and `ErrorKind::Argon2Failure`. Argon2 0.5.3 implements `std::error::Error` only behind its `std` feature, so that feature must also be enabled for the source-preserving constructor to accept the error.

## F5 / COR-TEST-KDF-COST

**Confidence:** possible

**Anchors:** `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474`

**Provenance:** correctness-state-lifecycle p1; correctness-systems p2. Two correctness lenses agreed. The merged confidence is the lower input tag.

The test module defines an 8 MiB Argon2 setting and says the 256 MiB production default is exercised only by `test_default_params_roundtrip`. Two other tests nevertheless call the public random `encrypt` path, which always selects the production defaults. `test_engine_roundtrip_through_armor` performs one default-cost derivation to encrypt and another to decrypt; `test_random_encryption_uses_fresh_salt_and_nonce` performs two more default-cost encryptions. Together with the intended encrypt/decrypt pair in `test_default_params_roundtrip`, an ordinary unit-test run performs six 256 MiB derivations.

Rust may run those three tests concurrently, creating a peak of at least three simultaneous workspaces, or roughly 768 MiB, while also repeating costly production-parameter work that one focused round trip already covers. Depending on runner resources, this can cause swapping, out-of-memory termination, or long test times. Keep `test_default_params_roundtrip` as the single production-default test, and give the armor round-trip and randomness tests a test-only route that uses the 8 MiB parameters while still generating fresh salt and nonce; the latter tests need to verify engine wiring and randomness, not production KDF cost.

Restated: 5/5 findings
```

## restated-attempt-2.md

```text
# Merged correctness findings

- `F1 / COR-DISPATCH` — **definite** — `src/lib.rs:8`, `src/format.rs:72-78` — saltybox2 is unavailable through the normal format dispatcher

The new `format_v2` module defines `V2Engine`, the component that knows how to encrypt and decrypt the new `saltybox2:` file format, and `src/lib.rs` makes that module public. The application does not call format engines directly, however. Its file operations go through the central format dispatcher: decryption calls `format::decode` to choose an engine from the input's magic prefix, while encryption and update call `format::default_write_engine` to choose the output format. The dispatcher's engine list contains only `V1Engine`, and its write default also returns `V1Engine`. Consequently, a `saltybox2:` file reaches the dispatcher's unsupported-version branch and is reported as coming from a future saltybox version, while newly encrypted files continue to use `saltybox1:`.

The new golden-vector tests do not expose this integration failure because they instantiate `V2Engine` directly. Register `V2Engine` in `format::ENGINES` so normal reads can select it, and make it the default write engine if this commit is intended to activate saltybox2 for new files. Add coverage through `format::decode` and the high-level file or CLI path so both saltybox2 reads and the intended output-format default are tested.

- `F2 / COR-KDF-BUDGET` — **possible** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352` — unauthenticated input can request an extreme Argon2 workload

A saltybox2 file stores its Argon2 key-derivation settings in an unauthenticated header. Decryption must derive the key before it can verify the file's authentication tag, so an attacker can choose these settings without knowing the passphrase or producing valid ciphertext. `validate_params` accepts memory cost, pass count, and lane count independently up to 4,194,304 KiB, 64 passes, and eight lanes. A payload needs only a complete header and a 16-byte dummy authentication tag to pass the structural checks and reach key derivation, yet the maximum accepted values ask Argon2 to allocate about 4 GiB and process that workspace repeatedly before authentication inevitably fails.

That accepted workload is unsafe on many otherwise supported machines: it can cause heavy swapping, prolonged CPU use, out-of-memory termination, or allocation failure before saltybox can return a normal input error. Argon2's `hash_password_into` creates its workspace with an infallible `vec!`, so the existing `Result` path does not turn inability to allocate the requested memory into a structured `SaltyboxError`. Replace the independent upper bounds with a defensible budget for unauthenticated decryption that accounts for the combined memory and iteration cost. If unusually expensive historical parameter sets must remain readable, require an explicit opt-in for them, and allocate caller-provided Argon2 workspace through a fallible allocation path so resource exhaustion is reported instead of terminating the process.

- `F3 / COR-KDF-ZEROIZE` — **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213` — Argon2's passphrase-derived workspace is freed without being erased

Argon2 deliberately fills a large memory workspace with state derived from the passphrase. `derive_key` protects the final 32-byte key with `Zeroizing`, but it calls Argon2's convenience `hash_password_into` method, which allocates the workspace internally as an ordinary vector and releases it when the call finishes. The new dependency declaration enables Argon2's default features, which do not include its optional `zeroize` support. At the 256 MiB write default, roughly 256 MiB of intermediate passphrase-derived blocks can therefore remain in allocator-managed heap pages after each encryption or decryption even though saltybox erases the passphrase, output key, and plaintext buffers it owns.

Whether that residue can be recovered depends on later allocator reuse and on an attacker's ability to inspect process memory, a crash dump, or another disclosure, so the practical exploitability is environment-dependent. The implementation should enable Argon2's `zeroize` feature, allocate the block workspace explicitly, wrap it in `Zeroizing`, and call `hash_password_into_with_memory`. That gives saltybox ownership of the workspace and erases the blocks on both successful and error returns before the allocation is released.

- `F4 / COR-ERROR-SOURCE` — **definite** — `src/format_v2.rs:193-211` — Argon2 failures lose their structured source error

Both Argon2 operations in `derive_key` can return `argon2::Error`: constructing the parameter set can reject invalid values, and hashing can reject invalid inputs or workspace conditions. Each `map_err` currently formats that error into the wrapper's message and then calls `SaltyboxError::with_kind`, whose source field is always `None`. Callers retain the broad internal-error category and `Argon2Failure` kind, but `std::error::Error::source()` cannot recover the original typed Argon2 error. When a higher-level file operation adds context, the CLI's cause reporter can walk as far as the source-less `SaltyboxError`, but it cannot display the Argon2 error as the underlying cause.

Preserve the existing stable wrapper messages and `ErrorKind::Argon2Failure`, but construct both errors with `SaltyboxError::with_kind_and_source` and pass the original `argon2::Error`. Argon2 0.5.3 implements `std::error::Error` only when its `std` feature is enabled, so enable that feature as part of the change if required by the wrapper's source bound. This restores typed error inspection for library users and the complete cause chain for CLI diagnostics.

- `F5 / COR-TEST-KDF-COST` — **possible** — `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474` — three parallel tests run the 256 MiB production KDF

The test module defines an 8 MiB Argon2 setting for routine coverage and says that only `test_default_params_roundtrip` should exercise the 256 MiB production default. Two other tests bypass that small setting. `test_engine_roundtrip_through_armor` calls the normal engine encrypt path and then decrypts its result, performing two production-cost derivations. `test_random_encryption_uses_fresh_salt_and_nonce` calls normal encryption twice, adding two more. Together with the encrypt and decrypt in `test_default_params_roundtrip`, these three tests perform six 256 MiB derivations in an ordinary unit-test run.

Rust may execute the three tests concurrently, allowing at least three 256 MiB workspaces, or roughly 768 MiB, to be live at once. On a constrained runner, that can produce swapping, long test times, or an out-of-memory failure, while only one of the tests needs to validate the real production parameters. Keep `test_default_params_roundtrip` as the single production-cost round trip. Give the armor round-trip and randomness tests a test-only route that still generates fresh random salts and nonces but accepts the 8 MiB test parameters, so they retain their behavioral coverage without duplicating the resource-heavy default.

Restated: 5/5 findings
```

## restated.md

```text
# Merged correctness findings

- `F1 / COR-DISPATCH` — **definite** — `src/lib.rs:8`, `src/format.rs:72-78` — saltybox2 is unavailable through the normal format dispatcher

The new `format_v2` module defines `V2Engine`, the component that knows how to encrypt and decrypt the new `saltybox2:` file format, and `src/lib.rs` makes that module public. The application does not call format engines directly, however. Its file operations go through the central format dispatcher: decryption calls `format::decode` to choose an engine from the input's magic prefix, while encryption and update call `format::default_write_engine` to choose the output format. The dispatcher's engine list contains only `V1Engine`, and its write default also returns `V1Engine`. Consequently, a `saltybox2:` file reaches the dispatcher's unsupported-version branch and is reported as coming from a future saltybox version, while newly encrypted files continue to use `saltybox1:`.

The new golden-vector tests do not expose this integration failure because they instantiate `V2Engine` directly. Register `V2Engine` in `format::ENGINES` so normal reads can select it, and make it the default write engine if this commit is intended to activate saltybox2 for new files. Add coverage through `format::decode` and the high-level file or CLI path so both saltybox2 reads and the intended output-format default are tested.

- `F2 / COR-KDF-BUDGET` — **possible** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352` — unauthenticated input can request an extreme Argon2 workload

A saltybox2 file stores its Argon2 key-derivation settings in an unauthenticated header. Decryption must derive the key before it can verify the file's authentication tag, so an attacker can choose these settings without knowing the passphrase or producing valid ciphertext. `validate_params` accepts memory cost, pass count, and lane count independently up to 4,194,304 KiB, 64 passes, and eight lanes. A payload needs only a complete header and a 16-byte dummy authentication tag to pass the structural checks and reach key derivation, yet the maximum accepted values ask Argon2 to allocate about 4 GiB and process that workspace repeatedly before authentication inevitably fails.

That accepted workload is unsafe on many otherwise supported machines: it can cause heavy swapping, prolonged CPU use, out-of-memory termination, or allocation failure before saltybox can return a normal input error. Argon2's `hash_password_into` creates its workspace with an infallible `vec!`, so the existing `Result` path does not turn inability to allocate the requested memory into a structured `SaltyboxError`. Replace the independent upper bounds with a defensible budget for unauthenticated decryption that accounts for the combined memory and iteration cost. If unusually expensive historical parameter sets must remain readable, require an explicit opt-in for them, and allocate caller-provided Argon2 workspace through a fallible allocation path so resource exhaustion is reported instead of terminating the process.

- `F3 / COR-KDF-ZEROIZE` — **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213` — Argon2's passphrase-derived workspace is freed without being erased

Argon2 deliberately fills a large memory workspace with state derived from the passphrase. `derive_key` protects the final 32-byte key with `Zeroizing`, but it calls Argon2's convenience `hash_password_into` method, which allocates the workspace internally as an ordinary vector and releases it when the call finishes. The new dependency declaration enables Argon2's default features, which do not include its optional `zeroize` support. At the 256 MiB write default, roughly 256 MiB of intermediate passphrase-derived blocks can therefore remain in allocator-managed heap pages after each encryption or decryption even though saltybox erases the passphrase, output key, and plaintext buffers it owns.

Whether that residue can be recovered depends on later allocator reuse and on an attacker's ability to inspect process memory, a crash dump, or another disclosure, so the practical exploitability is environment-dependent. The implementation should enable Argon2's `zeroize` feature, allocate the block workspace explicitly, wrap it in `Zeroizing`, and call `hash_password_into_with_memory`. That gives saltybox ownership of the workspace and erases the blocks on both successful and error returns before the allocation is released.

- `F4 / COR-ERROR-SOURCE` — **definite** — `src/format_v2.rs:193-211` — Argon2 failures lose their structured source error

Both Argon2 operations in `derive_key` can return `argon2::Error`: constructing the parameter set can reject invalid values, and hashing can reject invalid inputs or workspace conditions. Each `map_err` currently formats that error into the wrapper's message and then calls `SaltyboxError::with_kind`, whose source field is always `None`. Callers retain the broad internal-error category and `Argon2Failure` kind, but `std::error::Error::source()` cannot recover the original typed Argon2 error. When a higher-level file operation adds context, the CLI's cause reporter can walk as far as the source-less `SaltyboxError`, but it cannot display the Argon2 error as the underlying cause.

Preserve the existing stable wrapper messages and `ErrorKind::Argon2Failure`, but construct both errors with `SaltyboxError::with_kind_and_source` and pass the original `argon2::Error`. Argon2 0.5.3 implements `std::error::Error` only when its `std` feature is enabled, so enable that feature as part of the change if required by the wrapper's source bound. This restores typed error inspection for library users and the complete cause chain for CLI diagnostics.

- `F5 / COR-TEST-KDF-COST` — **possible** — `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474` — three parallel tests run the 256 MiB production KDF

The test module defines an 8 MiB Argon2 setting for routine coverage and says that only `test_default_params_roundtrip` should exercise the 256 MiB production default. Two other tests bypass that small setting. `test_engine_roundtrip_through_armor` calls the normal engine encrypt path and then decrypts its result, performing two production-cost derivations. `test_random_encryption_uses_fresh_salt_and_nonce` calls normal encryption twice, adding two more. Together with the encrypt and decrypt in `test_default_params_roundtrip`, these three tests perform six 256 MiB derivations in an ordinary unit-test run.

Rust may execute the three tests concurrently, allowing at least three 256 MiB workspaces, or roughly 768 MiB, to be live at once. On a constrained runner, that can produce swapping, long test times, or an out-of-memory failure, while only one of the tests needs to validate the real production parameters. Keep `test_default_params_roundtrip` as the single production-cost round trip. Give the armor round-trip and randomness tests a test-only route that still generates fresh random salts and nonces but accepts the 8 MiB test parameters, so they retain their behavioral coverage without duplicating the resource-heavy default.

Restated: 5/5 findings
```

## REPORT.md

```text
# Correctness-only pre-PR review

NOTE: The user restricted this evaluation arm to the five correctness reviewers. Other review categories were not run.

Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 5/5 reviewers completed; panel restricted to correctness by the user.

Reviewer continuation: 5/5 second passes, 2/2 third passes; unavailable: none; capped with new findings: correctness-general

Restatement: 5/5 findings restated, 2 attempts

Finding accounting: 12 reviewer findings → 5 reported (7 same-location merges, 0 rejected)

## Correctness

- `F1 / COR-DISPATCH` — **definite** — `src/lib.rs:8`, `src/format.rs:72-78` — saltybox2 is unavailable through the normal format dispatcher

The new `format_v2` module defines `V2Engine`, the component that knows how to encrypt and decrypt the new `saltybox2:` file format, and `src/lib.rs` makes that module public. The application does not call format engines directly, however. Its file operations go through the central format dispatcher: decryption calls `format::decode` to choose an engine from the input's magic prefix, while encryption and update call `format::default_write_engine` to choose the output format. The dispatcher's engine list contains only `V1Engine`, and its write default also returns `V1Engine`. Consequently, a `saltybox2:` file reaches the dispatcher's unsupported-version branch and is reported as coming from a future saltybox version, while newly encrypted files continue to use `saltybox1:`.

The new golden-vector tests do not expose this integration failure because they instantiate `V2Engine` directly. Register `V2Engine` in `format::ENGINES` so normal reads can select it, and make it the default write engine if this commit is intended to activate saltybox2 for new files. Add coverage through `format::decode` and the high-level file or CLI path so both saltybox2 reads and the intended output-format default are tested.

- `F2 / COR-KDF-BUDGET` — **possible** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352` — unauthenticated input can request an extreme Argon2 workload

A saltybox2 file stores its Argon2 key-derivation settings in an unauthenticated header. Decryption must derive the key before it can verify the file's authentication tag, so an attacker can choose these settings without knowing the passphrase or producing valid ciphertext. `validate_params` accepts memory cost, pass count, and lane count independently up to 4,194,304 KiB, 64 passes, and eight lanes. A payload needs only a complete header and a 16-byte dummy authentication tag to pass the structural checks and reach key derivation, yet the maximum accepted values ask Argon2 to allocate about 4 GiB and process that workspace repeatedly before authentication inevitably fails.

That accepted workload is unsafe on many otherwise supported machines: it can cause heavy swapping, prolonged CPU use, out-of-memory termination, or allocation failure before saltybox can return a normal input error. Argon2's `hash_password_into` creates its workspace with an infallible `vec!`, so the existing `Result` path does not turn inability to allocate the requested memory into a structured `SaltyboxError`. Replace the independent upper bounds with a defensible budget for unauthenticated decryption that accounts for the combined memory and iteration cost. If unusually expensive historical parameter sets must remain readable, require an explicit opt-in for them, and allocate caller-provided Argon2 workspace through a fallible allocation path so resource exhaustion is reported instead of terminating the process.

- `F3 / COR-KDF-ZEROIZE` — **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213` — Argon2's passphrase-derived workspace is freed without being erased

Argon2 deliberately fills a large memory workspace with state derived from the passphrase. `derive_key` protects the final 32-byte key with `Zeroizing`, but it calls Argon2's convenience `hash_password_into` method, which allocates the workspace internally as an ordinary vector and releases it when the call finishes. The new dependency declaration enables Argon2's default features, which do not include its optional `zeroize` support. At the 256 MiB write default, roughly 256 MiB of intermediate passphrase-derived blocks can therefore remain in allocator-managed heap pages after each encryption or decryption even though saltybox erases the passphrase, output key, and plaintext buffers it owns.

Whether that residue can be recovered depends on later allocator reuse and on an attacker's ability to inspect process memory, a crash dump, or another disclosure, so the practical exploitability is environment-dependent. The implementation should enable Argon2's `zeroize` feature, allocate the block workspace explicitly, wrap it in `Zeroizing`, and call `hash_password_into_with_memory`. That gives saltybox ownership of the workspace and erases the blocks on both successful and error returns before the allocation is released.

- `F4 / COR-ERROR-SOURCE` — **definite** — `src/format_v2.rs:193-211` — Argon2 failures lose their structured source error

Both Argon2 operations in `derive_key` can return `argon2::Error`: constructing the parameter set can reject invalid values, and hashing can reject invalid inputs or workspace conditions. Each `map_err` currently formats that error into the wrapper's message and then calls `SaltyboxError::with_kind`, whose source field is always `None`. Callers retain the broad internal-error category and `Argon2Failure` kind, but `std::error::Error::source()` cannot recover the original typed Argon2 error. When a higher-level file operation adds context, the CLI's cause reporter can walk as far as the source-less `SaltyboxError`, but it cannot display the Argon2 error as the underlying cause.

Preserve the existing stable wrapper messages and `ErrorKind::Argon2Failure`, but construct both errors with `SaltyboxError::with_kind_and_source` and pass the original `argon2::Error`. Argon2 0.5.3 implements `std::error::Error` only when its `std` feature is enabled, so enable that feature as part of the change if required by the wrapper's source bound. This restores typed error inspection for library users and the complete cause chain for CLI diagnostics.

- `F5 / COR-TEST-KDF-COST` — **possible** — `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474` — three parallel tests run the 256 MiB production KDF

The test module defines an 8 MiB Argon2 setting for routine coverage and says that only `test_default_params_roundtrip` should exercise the 256 MiB production default. Two other tests bypass that small setting. `test_engine_roundtrip_through_armor` calls the normal engine encrypt path and then decrypts its result, performing two production-cost derivations. `test_random_encryption_uses_fresh_salt_and_nonce` calls normal encryption twice, adding two more. Together with the encrypt and decrypt in `test_default_params_roundtrip`, these three tests perform six 256 MiB derivations in an ordinary unit-test run.

Rust may execute the three tests concurrently, allowing at least three 256 MiB workspaces, or roughly 768 MiB, to be live at once. On a constrained runner, that can produce swapping, long test times, or an out-of-memory failure, while only one of the tests needs to validate the real production parameters. Keep `test_default_params_roundtrip` as the single production-cost round trip. Give the armor round-trip and randomness tests a test-only route that still generates fresh random salts and nonces but accepts the 8 MiB test parameters, so they retain their behavioral coverage without duplicating the resource-heavy default.

Restated: 5/5 findings
```

## BUCKETS.md

```text
# Nofix confirmation and buckets

No code changes were applied. These are the decisions the normal fixing mode would make after checking each finding against the reviewed checkout and the pinned Argon2 0.5.3 dependency source.

### F1 / COR-DISPATCH

- Category and origin: correctness; correctness-edge-inputs p1
- Confidence: definite as filed; no retag
- Bucket: would reject
- Reason: The observed dispatcher state is real, but it is the explicit boundary of this commit rather than a defect. The commit is titled `chore: add saltybox2 format engine (unwired)`, and the checked-in step-4 acceptance criteria say not to register v2 or touch the CLI because this unit must have no user-visible change. Wiring belongs to a later review unit.

### F2 / COR-KDF-BUDGET

- Category and origin: correctness; correctness-general p1, with same-location agreement from correctness-data-flow p1, correctness-state-lifecycle p1, correctness-systems p1, and correctness-edge-inputs p1
- Confidence: possible as merged; no retag
- Bucket: would surface
- Reason: The code and dependency confirm that an unauthenticated header may request the accepted 4 GiB, 64-pass combination before authentication, through an infallible workspace allocation. The effect depends on host resources, and the exact caps are part of the planned on-disk format contract. Choosing a lower budget, a combined-work formula, and an opt-in compatibility path is a design decision with externally visible compatibility consequences.
- Question: What automatic-decrypt resource budget and compatibility policy should saltybox2 use before the engine is wired into normal reads?

### F3 / COR-KDF-ZEROIZE

- Category and origin: correctness; correctness-systems p1, with same-location agreement from correctness-state-lifecycle p1 and correctness-general p2
- Confidence: possible as merged; no retag
- Bucket: would fix
- Reason: Argon2 0.5.3's convenience method allocates an ordinary `Vec<Block>` and does not erase it; the optional feature only gives `Block` a `Zeroize` implementation. The local fix is to enable that feature and pass an explicitly allocated `Zeroizing` workspace to `hash_password_into_with_memory`. This is a local, non-structural change below the possible-finding size gate.
- Overlap: The Cargo feature change overlaps mechanically with F4's need for Argon2 `std` support, so normal mode would combine the dependency feature edit while keeping the source changes attributable to each finding.

### F4 / COR-ERROR-SOURCE

- Category and origin: correctness; correctness-general p3
- Confidence: definite as filed; no retag
- Bucket: would fix
- Reason: Both Argon2 errors are converted to text and wrapped with a constructor that stores no source, while the crate's error type and CLI explicitly support source chains. Argon2 0.5.3 provides `std::error::Error` behind its `std` feature. Enabling that feature and using `with_kind_and_source` at the two local call sites is the smallest complete fix.
- Overlap: The Cargo feature edit overlaps mechanically with F3.

### F5 / COR-TEST-KDF-COST

- Category and origin: correctness; correctness-state-lifecycle p1 and correctness-systems p2
- Confidence: possible as merged; no retag
- Bucket: would fix
- Reason: The checked-in plan and the test comment both require most tests to use small parameters and only one round trip to use production defaults. The three named tests instead perform six 256 MiB derivations and may run concurrently. A test-only small-parameter encryption path is eligible under the test-code exception and preserves the tests' intended assertions without changing shipped behavior.
```

## RUN-LOG.md

```text
# Pre-PR Review Swarm Run: 20260905-0442-62d866d-4984-baseline

- Timestamp: 2026-09-05T05:06:22Z
- Repository root: `EVAL_ROOT/baseline/repo`
- Commit: `62d866d6a57a24ef6bb329b28a246b44b758ff7a`
- Scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)
- Skill arguments: `nofix commit`
- Coordinator model: `gpt-5.6-sol` with `high` reasoning effort
- Panel: user-restricted correctness reviewers only

## Findings report

# Correctness-only pre-PR review

NOTE: The user restricted this evaluation arm to the five correctness reviewers. Other review categories were not run.

Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 5/5 reviewers completed; panel restricted to correctness by the user.

Reviewer continuation: 5/5 second passes, 2/2 third passes; unavailable: none; capped with new findings: correctness-general

Restatement: 5/5 findings restated, 2 attempts

Finding accounting: 12 reviewer findings → 5 reported (7 same-location merges, 0 rejected)

## Correctness

- `F1 / COR-DISPATCH` — **definite** — `src/lib.rs:8`, `src/format.rs:72-78` — saltybox2 is unavailable through the normal format dispatcher

The new `format_v2` module defines `V2Engine`, the component that knows how to encrypt and decrypt the new `saltybox2:` file format, and `src/lib.rs` makes that module public. The application does not call format engines directly, however. Its file operations go through the central format dispatcher: decryption calls `format::decode` to choose an engine from the input's magic prefix, while encryption and update call `format::default_write_engine` to choose the output format. The dispatcher's engine list contains only `V1Engine`, and its write default also returns `V1Engine`. Consequently, a `saltybox2:` file reaches the dispatcher's unsupported-version branch and is reported as coming from a future saltybox version, while newly encrypted files continue to use `saltybox1:`.

The new golden-vector tests do not expose this integration failure because they instantiate `V2Engine` directly. Register `V2Engine` in `format::ENGINES` so normal reads can select it, and make it the default write engine if this commit is intended to activate saltybox2 for new files. Add coverage through `format::decode` and the high-level file or CLI path so both saltybox2 reads and the intended output-format default are tested.

- `F2 / COR-KDF-BUDGET` — **possible** — `src/format_v2.rs:69-81`, `src/format_v2.rs:131-179`, `src/format_v2.rs:351-352` — unauthenticated input can request an extreme Argon2 workload

A saltybox2 file stores its Argon2 key-derivation settings in an unauthenticated header. Decryption must derive the key before it can verify the file's authentication tag, so an attacker can choose these settings without knowing the passphrase or producing valid ciphertext. `validate_params` accepts memory cost, pass count, and lane count independently up to 4,194,304 KiB, 64 passes, and eight lanes. A payload needs only a complete header and a 16-byte dummy authentication tag to pass the structural checks and reach key derivation, yet the maximum accepted values ask Argon2 to allocate about 4 GiB and process that workspace repeatedly before authentication inevitably fails.

That accepted workload is unsafe on many otherwise supported machines: it can cause heavy swapping, prolonged CPU use, out-of-memory termination, or allocation failure before saltybox can return a normal input error. Argon2's `hash_password_into` creates its workspace with an infallible `vec!`, so the existing `Result` path does not turn inability to allocate the requested memory into a structured `SaltyboxError`. Replace the independent upper bounds with a defensible budget for unauthenticated decryption that accounts for the combined memory and iteration cost. If unusually expensive historical parameter sets must remain readable, require an explicit opt-in for them, and allocate caller-provided Argon2 workspace through a fallible allocation path so resource exhaustion is reported instead of terminating the process.

- `F3 / COR-KDF-ZEROIZE` — **possible** — `Cargo.toml:18`, `src/format_v2.rs:182-213` — Argon2's passphrase-derived workspace is freed without being erased

Argon2 deliberately fills a large memory workspace with state derived from the passphrase. `derive_key` protects the final 32-byte key with `Zeroizing`, but it calls Argon2's convenience `hash_password_into` method, which allocates the workspace internally as an ordinary vector and releases it when the call finishes. The new dependency declaration enables Argon2's default features, which do not include its optional `zeroize` support. At the 256 MiB write default, roughly 256 MiB of intermediate passphrase-derived blocks can therefore remain in allocator-managed heap pages after each encryption or decryption even though saltybox erases the passphrase, output key, and plaintext buffers it owns.

Whether that residue can be recovered depends on later allocator reuse and on an attacker's ability to inspect process memory, a crash dump, or another disclosure, so the practical exploitability is environment-dependent. The implementation should enable Argon2's `zeroize` feature, allocate the block workspace explicitly, wrap it in `Zeroizing`, and call `hash_password_into_with_memory`. That gives saltybox ownership of the workspace and erases the blocks on both successful and error returns before the allocation is released.

- `F4 / COR-ERROR-SOURCE` — **definite** — `src/format_v2.rs:193-211` — Argon2 failures lose their structured source error

Both Argon2 operations in `derive_key` can return `argon2::Error`: constructing the parameter set can reject invalid values, and hashing can reject invalid inputs or workspace conditions. Each `map_err` currently formats that error into the wrapper's message and then calls `SaltyboxError::with_kind`, whose source field is always `None`. Callers retain the broad internal-error category and `Argon2Failure` kind, but `std::error::Error::source()` cannot recover the original typed Argon2 error. When a higher-level file operation adds context, the CLI's cause reporter can walk as far as the source-less `SaltyboxError`, but it cannot display the Argon2 error as the underlying cause.

Preserve the existing stable wrapper messages and `ErrorKind::Argon2Failure`, but construct both errors with `SaltyboxError::with_kind_and_source` and pass the original `argon2::Error`. Argon2 0.5.3 implements `std::error::Error` only when its `std` feature is enabled, so enable that feature as part of the change if required by the wrapper's source bound. This restores typed error inspection for library users and the complete cause chain for CLI diagnostics.

- `F5 / COR-TEST-KDF-COST` — **possible** — `src/format_v2.rs:379-381`, `src/format_v2.rs:434-474` — three parallel tests run the 256 MiB production KDF

The test module defines an 8 MiB Argon2 setting for routine coverage and says that only `test_default_params_roundtrip` should exercise the 256 MiB production default. Two other tests bypass that small setting. `test_engine_roundtrip_through_armor` calls the normal engine encrypt path and then decrypts its result, performing two production-cost derivations. `test_random_encryption_uses_fresh_salt_and_nonce` calls normal encryption twice, adding two more. Together with the encrypt and decrypt in `test_default_params_roundtrip`, these three tests perform six 256 MiB derivations in an ordinary unit-test run.

Rust may execute the three tests concurrently, allowing at least three 256 MiB workspaces, or roughly 768 MiB, to be live at once. On a constrained runner, that can produce swapping, long test times, or an out-of-memory failure, while only one of the tests needs to validate the real production parameters. Keep `test_default_params_roundtrip` as the single production-cost round trip. Give the armor round-trip and randomness tests a test-only route that still generates fresh random salts and nonces but accepts the 8 MiB test parameters, so they retain their behavioral coverage without duplicating the resource-heavy default.

Restated: 5/5 findings

## Finding decisions

# Nofix confirmation and buckets

No code changes were applied. These are the decisions the normal fixing mode would make after checking each finding against the reviewed checkout and the pinned Argon2 0.5.3 dependency source.

### F1 / COR-DISPATCH

- Category and origin: correctness; correctness-edge-inputs p1
- Confidence: definite as filed; no retag
- Bucket: would reject
- Reason: The observed dispatcher state is real, but it is the explicit boundary of this commit rather than a defect. The commit is titled `chore: add saltybox2 format engine (unwired)`, and the checked-in step-4 acceptance criteria say not to register v2 or touch the CLI because this unit must have no user-visible change. Wiring belongs to a later review unit.

### F2 / COR-KDF-BUDGET

- Category and origin: correctness; correctness-general p1, with same-location agreement from correctness-data-flow p1, correctness-state-lifecycle p1, correctness-systems p1, and correctness-edge-inputs p1
- Confidence: possible as merged; no retag
- Bucket: would surface
- Reason: The code and dependency confirm that an unauthenticated header may request the accepted 4 GiB, 64-pass combination before authentication, through an infallible workspace allocation. The effect depends on host resources, and the exact caps are part of the planned on-disk format contract. Choosing a lower budget, a combined-work formula, and an opt-in compatibility path is a design decision with externally visible compatibility consequences.
- Question: What automatic-decrypt resource budget and compatibility policy should saltybox2 use before the engine is wired into normal reads?

### F3 / COR-KDF-ZEROIZE

- Category and origin: correctness; correctness-systems p1, with same-location agreement from correctness-state-lifecycle p1 and correctness-general p2
- Confidence: possible as merged; no retag
- Bucket: would fix
- Reason: Argon2 0.5.3's convenience method allocates an ordinary `Vec<Block>` and does not erase it; the optional feature only gives `Block` a `Zeroize` implementation. The local fix is to enable that feature and pass an explicitly allocated `Zeroizing` workspace to `hash_password_into_with_memory`. This is a local, non-structural change below the possible-finding size gate.
- Overlap: The Cargo feature change overlaps mechanically with F4's need for Argon2 `std` support, so normal mode would combine the dependency feature edit while keeping the source changes attributable to each finding.

### F4 / COR-ERROR-SOURCE

- Category and origin: correctness; correctness-general p3
- Confidence: definite as filed; no retag
- Bucket: would fix
- Reason: Both Argon2 errors are converted to text and wrapped with a constructor that stores no source, while the crate's error type and CLI explicitly support source chains. Argon2 0.5.3 provides `std::error::Error` behind its `std` feature. Enabling that feature and using `with_kind_and_source` at the two local call sites is the smallest complete fix.
- Overlap: The Cargo feature edit overlaps mechanically with F3.

### F5 / COR-TEST-KDF-COST

- Category and origin: correctness; correctness-state-lifecycle p1 and correctness-systems p2
- Confidence: possible as merged; no retag
- Bucket: would fix
- Reason: The checked-in plan and the test comment both require most tests to use small parameters and only one round trip to use production defaults. The three named tests instead perform six 256 MiB derivations and may run concurrently. A test-only small-parameter encryption path is eligible under the test-code exception and preserves the tests' intended assertions without changing shipped behavior.
```
