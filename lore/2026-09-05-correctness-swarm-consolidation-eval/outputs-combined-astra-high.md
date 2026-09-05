# combined-astra-high: original review outputs

These are the recorded outputs, including incorrect or disputed claims. The parent assessment is in
[FINDINGS.md](FINDINGS.md). Local paths and native identifiers are replaced; finding wording is retained. Source paths
refer to the pinned saltybox commit.

## correctness-combined-p1.md

```text
Reviewed `current commit 62d866d (9 touched files, 958 diff lines; correctness only)`: completed the general full-scope pass, dedicated data/error-flow, state/lifecycle, systems/resource-bounds, and boundary/adversarial-input passes, and a final sweep for unrelated correctness issues. No builds or tests were run.

- **definite — Wipe the Argon2 workspace before releasing it.** Source anchor: `src/format_v2.rs:202–204` (`derive_key` calling `hash_password_into`).

  What happens: The new key-derivation helper protects its 32-byte output with `Zeroizing`, but calls an Argon2 API that allocates its own ordinary `Vec<Block>`. In the pinned argon2 0.5.3 implementation, `hash_password_into` allocates that vector at `src/lib.rs:229–231` and releases it without clearing it; `Block` has no drop-time wipe. This happens on the encryption and decryption paths. At the default parameters, the workspace contains 256 MiB of password-derived state. Argon2's finalization at dependency `src/lib.rs:481–499` derives the encryption key from the final block of each lane, so this allocation contains key-recoverable material, not merely harmless intermediate bookkeeping.

  Why it matters: Clearing the returned key does not remove the equivalent secret remaining in the released workspace. If those bytes are subsequently exposed through a memory disclosure or captured memory, the final lane blocks allow reconstruction of the encryption key without guessing the passphrase or repeating the expensive derivation. The new engine therefore leaves sensitive derivation state outside its intended secret-buffer cleanup.

  Suggested change: Allocate an explicitly zeroizing Argon2 block workspace in `derive_key` and call `hash_password_into_with_memory`, ensuring the workspace is wiped on both success and error. Enable Argon2's `zeroize` feature to obtain its `Block` zeroization support. Enabling the feature alone does not fix this allocation: the convenience `hash_password_into` API still uses an ordinary vector.

- **definite — Enable Argon2's cleanup of internal secret temporaries.** Source anchor: `Cargo.toml:18` (`argon2 = "0.5"`).

  What happens: The new Argon2 dependency uses its default features, which omit `zeroize`; the frozen lockfile also confirms that Argon2 has no zeroize dependency enabled. In argon2 0.5.3, erasure of the initial password-derived hash at dependency `src/lib.rs:322–323` and the final `blockhash`/`blockhash_bytes` temporaries at `src/lib.rs:501–505` is conditional on that feature. Consequently, these internal temporaries are not explicitly erased when the new engine derives a key, even though its own output key is wrapped in `Zeroizing`.

  Why it matters: The final block temporaries are sufficient to reproduce the output key through Argon2's final hash. They are separate local copies, so wiping the caller's key and even fixing the heap-workspace cleanup above does not wipe them. This leaves another source of recoverable encryption-key material after derivation returns.

  Suggested change: Declare Argon2 with `features = ["zeroize"]` so its existing internal cleanup executes. Keep this enabled alongside explicit workspace cleanup; the two changes protect different allocations and neither substitutes for the other.
```

## correctness-combined-p2.md

```text
Reviewed the complete frozen scope again with the combined correctness charter, using the existing review context and a bounded additional search. This pass reports only one new finding; the two pass-1 findings are not repeated. No builds or tests were run.

- **possible — Reject Argon2 workspace sizes that cannot fit the target address space.** Source anchors: `src/format_v2.rs:160–169` (memory validation), `src/format_v2.rs:203–204` (allocating Argon2 call).

  What happens: Memory validation accepts `m = 4194304` KiB, `t = 1`, and `p = 1` on every architecture. A 68-byte payload containing those parameters and a dummy authentication tag reaches key derivation before authentication. The pinned argon2 0.5.3 `hash_password_into` implementation (`src/lib.rs:229–231`) then constructs a vector of 4,194,304 blocks, each 1,024 bytes. On a 32-bit build this 4 GiB allocation cannot be represented and exceeds Rust's maximum vector allocation size; it panics for capacity overflow instead of returning an error through the surrounding `map_err`. This does not require a valid passphrase or authentication tag.

  Why it matters: If 32-bit source builds are supported, a tiny malformed encrypted input can terminate the operation through a panic instead of producing the structured format/resource error expected from the validation boundary. The older engine already explicitly guards platform allocation limits in `src/secretcrypt_v1.rs:181–190`. The unresolved premise is platform support: `dist-workspace.toml:16` lists only 64-bit release targets, and no 32-bit build was run, so this finding does not claim that the published binaries hit this deterministic overflow.

  Suggested change: If source builds on 32-bit targets remain supported, check the required Argon2 block allocation against the current platform before calling the allocating API. Use checked multiplication of `params.block_count()` by the block size and reject sizes above `isize::MAX` with a structured error. This preserves the format's 4 GiB ceiling on capable targets while rejecting impossible allocations on smaller ones. Otherwise, make the 64-bit-only target requirement explicit and enforce it.
```

## correctness-combined-p3.md

```text
[]

I reviewed the complete scope a final time and found no new findings.
```

## merged.md

```text
## Correctness

- `F1 / COR-ARGON2-WORKSPACE` — **definite** — `src/format_v2.rs:202` — Wipe the Argon2 workspace before releasing it.

  What happens: The new key-derivation helper protects its 32-byte output with `Zeroizing`, but calls an Argon2 API that allocates its own ordinary `Vec<Block>`. In the pinned argon2 0.5.3 implementation, `hash_password_into` allocates that vector at dependency `src/lib.rs:229–231` and releases it without clearing it; `Block` has no drop-time wipe. This happens on the encryption and decryption paths. At the default parameters, the workspace contains 256 MiB of password-derived state. Argon2's finalization at dependency `src/lib.rs:481–499` derives the encryption key from the final block of each lane, so this allocation contains key-recoverable material, not merely harmless intermediate bookkeeping.

  Why it matters: Clearing the returned key does not remove the equivalent secret remaining in the released workspace. If those bytes are subsequently exposed through a memory disclosure or captured memory, the final lane blocks allow reconstruction of the encryption key without guessing the passphrase or repeating the expensive derivation. The new engine therefore leaves sensitive derivation state outside its intended secret-buffer cleanup.

  Suggested change: Allocate an explicitly zeroizing Argon2 block workspace in `derive_key` and call `hash_password_into_with_memory`, ensuring the workspace is wiped on both success and error. Enable Argon2's `zeroize` feature to obtain its `Block` zeroization support. Enabling the feature alone does not fix this allocation: the convenience `hash_password_into` API still uses an ordinary vector.

- `F2 / COR-ARGON2-TEMPORARIES` — **definite** — `Cargo.toml:18` — Enable Argon2's cleanup of internal secret temporaries.

  What happens: The new Argon2 dependency uses its default features, which omit `zeroize`; the frozen lockfile also confirms that Argon2 has no zeroize dependency enabled. In argon2 0.5.3, erasure of the initial password-derived hash at dependency `src/lib.rs:322–323` and the final `blockhash`/`blockhash_bytes` temporaries at `src/lib.rs:501–505` is conditional on that feature. Consequently, these internal temporaries are not explicitly erased when the new engine derives a key, even though its own output key is wrapped in `Zeroizing`.

  Why it matters: The final block temporaries are sufficient to reproduce the output key through Argon2's final hash. They are separate local copies, so wiping the caller's key and even fixing the heap-workspace cleanup above does not wipe them. This leaves another source of recoverable encryption-key material after derivation returns.

  Suggested change: Declare Argon2 with `features = ["zeroize"]` so its existing internal cleanup executes. Keep this enabled alongside explicit workspace cleanup; the two changes protect different allocations and neither substitutes for the other.

- `F3 / COR-ADDRSPACE` — **possible** — `src/format_v2.rs:160` — Reject Argon2 workspace sizes that cannot fit the target address space.

  What happens: Memory validation accepts `m = 4194304` KiB, `t = 1`, and `p = 1` on every architecture. A 68-byte payload containing those parameters and a dummy authentication tag reaches key derivation before authentication. The pinned argon2 0.5.3 `hash_password_into` implementation at dependency `src/lib.rs:229–231` then constructs a vector of 4,194,304 blocks, each 1,024 bytes. On a 32-bit build this 4 GiB allocation cannot be represented and exceeds Rust's maximum vector allocation size; it panics for capacity overflow instead of returning an error through the surrounding `map_err`. This does not require a valid passphrase or authentication tag.

  Why it matters: If 32-bit source builds are supported, a tiny malformed encrypted input can terminate the operation through a panic instead of producing the structured format/resource error expected from the validation boundary. The older engine already explicitly guards platform allocation limits in `src/secretcrypt_v1.rs:181–190`. The unresolved premise is platform support: `dist-workspace.toml:16` lists only 64-bit release targets, and no 32-bit build was run, so this finding does not claim that the published binaries hit this deterministic overflow.

  Suggested change: If source builds on 32-bit targets remain supported, check the required Argon2 block allocation against the current platform before calling the allocating API. Use checked multiplication of `params.block_count()` by the block size and reject sizes above `isize::MAX` with a structured error. This preserves the format's 4 GiB ceiling on capable targets while rejecting impossible allocations on smaller ones. Otherwise, make the 64-bit-only target requirement explicit and enforce it.
```

## restated.md

```text
## Correctness

- `F1 / COR-ARGON2-WORKSPACE` — **definite** — `src/format_v2.rs:202` — Wipe the Argon2 workspace before releasing it.

  Argon2 is a memory-hard key-derivation function: while turning the passphrase into an encryption key, it fills a large workspace with password-derived blocks. `derive_key` wraps only the final 32-byte key in `Zeroizing`, then calls Argon2 0.5.3's `hash_password_into` convenience method. That method allocates a normal `Vec<Block>` internally and drops it without clearing its contents. Both saltybox2 encryption and decryption reach this helper, and the default parameters make the abandoned workspace 256 MiB.

  The workspace contains more than disposable bookkeeping. Argon2 computes the returned key by combining the final block from each lane and hashing that value, so anyone who later obtains those released blocks through a memory disclosure or memory capture can reconstruct the encryption key without knowing the passphrase or paying the cost of deriving it again. Wiping the returned key therefore leaves equivalent key material behind.

  Change `derive_key` to allocate the block workspace itself in an explicitly zeroizing container and pass it to `hash_password_into_with_memory`. The container must wipe the allocation whether derivation succeeds or returns an error. Argon2's `zeroize` feature must also be enabled so its public `Block` type implements the cleanup trait required for that container. Enabling the feature by itself is insufficient because `hash_password_into` still creates and releases an ordinary vector.

- `F2 / COR-ARGON2-TEMPORARIES` — **definite** — `Cargo.toml:18` — Enable Argon2's cleanup of internal secret temporaries.

  The new `argon2 = "0.5"` dependency uses Argon2's default feature set, which does not include `zeroize`; the lockfile likewise shows that Argon2 has no dependency on the `zeroize` crate. In Argon2 0.5.3, cleanup of several internal values is compiled only when that feature is active. Without it, the initial hash derived from the password and the final `blockhash` and 1 KiB `blockhash_bytes` buffers remain uncleared when key derivation returns.

  The final two buffers are direct inputs to the last hash that produces the encryption key, so they provide enough material to calculate that key. They are independent copies from both the returned 32-byte key and the large workspace described above; wiping either of those other allocations does not clear these stack temporaries.

  Declare the Argon2 dependency with `features = ["zeroize"]` so the dependency's existing conditional cleanup runs. Keep that feature enabled in addition to explicitly zeroizing the workspace from F1, since the two changes erase different copies of the secret derivation state and neither replaces the other.

- `F3 / COR-ADDRSPACE` — **possible** — `src/format_v2.rs:160` — Reject Argon2 workspace sizes that cannot fit the target address space.

  `validate_params` accepts a memory cost of `4194304` KiB with one pass and one lane on every target because that value is exactly the format's configured upper limit. A hostile 68-byte binary payload can put those values in its 52-byte header and supply only a dummy 16-byte authentication tag. Decryption validates the header and starts key derivation before it authenticates the payload, so no valid passphrase or tag is needed to reach the allocation.

  With those parameters, Argon2 0.5.3 asks `Vec` for 4,194,304 blocks of 1,024 bytes each: exactly 4 GiB. That allocation cannot fit a 32-bit address space and exceeds Rust's maximum vector allocation size, causing a capacity-overflow panic before Argon2 can return an error. The surrounding `map_err` handles only returned Argon2 errors, so it cannot convert this panic into saltybox's structured format or resource error. If 32-bit source builds are supported, a very small malformed encrypted input can therefore terminate the operation.

  Before calling the allocating Argon2 API, use checked multiplication on `params.block_count()` and `Block::SIZE`, then reject any required byte count above the current platform's `isize::MAX` with a structured error. This keeps the format's 4 GiB limit available on address spaces that can support it while rejecting impossible allocations on smaller targets. The existing v1 engine already applies this kind of platform-specific `isize::MAX` guard to an input length. If 32-bit source builds are outside the supported platform set instead, document and enforce that requirement. The release configuration currently names only 64-bit targets, which is why this finding remains conditional rather than claiming that shipped binaries encounter the overflow.

Restated: 3/3 findings
```

## REPORT.md

```text
Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 1/1 reviewers completed; the user restricted this evaluation panel to correctness, so no other review categories ran.

Reviewer continuation: 1/1 second passes, 1/1 third passes; unavailable: none; capped with new findings: none

Restatement: 3/3 findings restated, 1 attempt(s)

Finding accounting: 3 reviewer findings → 3 reported (0 same-location merges, 0 rejected)

## Correctness

- `F1 / COR-ARGON2-WORKSPACE` — **definite** — `src/format_v2.rs:202` — Wipe the Argon2 workspace before releasing it.

  Argon2 is a memory-hard key-derivation function: while turning the passphrase into an encryption key, it fills a large workspace with password-derived blocks. `derive_key` wraps only the final 32-byte key in `Zeroizing`, then calls Argon2 0.5.3's `hash_password_into` convenience method. That method allocates a normal `Vec<Block>` internally and drops it without clearing its contents. Both saltybox2 encryption and decryption reach this helper, and the default parameters make the abandoned workspace 256 MiB.

  The workspace contains more than disposable bookkeeping. Argon2 computes the returned key by combining the final block from each lane and hashing that value, so anyone who later obtains those released blocks through a memory disclosure or memory capture can reconstruct the encryption key without knowing the passphrase or paying the cost of deriving it again. Wiping the returned key therefore leaves equivalent key material behind.

  Change `derive_key` to allocate the block workspace itself in an explicitly zeroizing container and pass it to `hash_password_into_with_memory`. The container must wipe the allocation whether derivation succeeds or returns an error. Argon2's `zeroize` feature must also be enabled so its public `Block` type implements the cleanup trait required for that container. Enabling the feature by itself is insufficient because `hash_password_into` still creates and releases an ordinary vector.

- `F2 / COR-ARGON2-TEMPORARIES` — **definite** — `Cargo.toml:18` — Enable Argon2's cleanup of internal secret temporaries.

  The new `argon2 = "0.5"` dependency uses Argon2's default feature set, which does not include `zeroize`; the lockfile likewise shows that Argon2 has no dependency on the `zeroize` crate. In Argon2 0.5.3, cleanup of several internal values is compiled only when that feature is active. Without it, the initial hash derived from the password and the final `blockhash` and 1 KiB `blockhash_bytes` buffers remain uncleared when key derivation returns.

  The final two buffers are direct inputs to the last hash that produces the encryption key, so they provide enough material to calculate that key. They are independent copies from both the returned 32-byte key and the large workspace described above; wiping either of those other allocations does not clear these stack temporaries.

  Declare the Argon2 dependency with `features = ["zeroize"]` so the dependency's existing conditional cleanup runs. Keep that feature enabled in addition to explicitly zeroizing the workspace from F1, since the two changes erase different copies of the secret derivation state and neither replaces the other.

- `F3 / COR-ADDRSPACE` — **possible** — `src/format_v2.rs:160` — Reject Argon2 workspace sizes that cannot fit the target address space.

  `validate_params` accepts a memory cost of `4194304` KiB with one pass and one lane on every target because that value is exactly the format's configured upper limit. A hostile 68-byte binary payload can put those values in its 52-byte header and supply only a dummy 16-byte authentication tag. Decryption validates the header and starts key derivation before it authenticates the payload, so no valid passphrase or tag is needed to reach the allocation.

  With those parameters, Argon2 0.5.3 asks `Vec` for 4,194,304 blocks of 1,024 bytes each: exactly 4 GiB. That allocation cannot fit a 32-bit address space and exceeds Rust's maximum vector allocation size, causing a capacity-overflow panic before Argon2 can return an error. The surrounding `map_err` handles only returned Argon2 errors, so it cannot convert this panic into saltybox's structured format or resource error. If 32-bit source builds are supported, a very small malformed encrypted input can therefore terminate the operation.

  Before calling the allocating Argon2 API, use checked multiplication on `params.block_count()` and `Block::SIZE`, then reject any required byte count above the current platform's `isize::MAX` with a structured error. This keeps the format's 4 GiB limit available on address spaces that can support it while rejecting impossible allocations on smaller targets. The existing v1 engine already applies this kind of platform-specific `isize::MAX` guard to an input length. If 32-bit source builds are outside the supported platform set instead, document and enforce that requirement. The release configuration currently names only 64-bit targets, which is why this finding remains conditional rather than claiming that shipped binaries encounter the overflow.
```

## BUCKETS.md

```text
# Nofix bucket assignment

No source changes were made. These are the buckets the default fixing mode would use after confirming each finding against the pinned checkout and the on-disk Argon2 0.5.3 source.

### F1 / COR-ARGON2-WORKSPACE

- Category and origin: correctness, `correctness-combined p1`
- Confidence: filed **definite**; confirmed **definite**
- Bucket: **would fix**
- Reason: `derive_key` calls `hash_password_into`, which allocates an ordinary `Vec<Block>`. Argon2 finalization derives the output from the final lane blocks still held by that vector, and neither the vector nor `Block` has active drop-time wiping under the current features. This is a confirmed correctness finding with a complete implementation fix that does not require a public API, format, or policy decision.
- Larger proposal not applied: none. The explicit zeroizing workspace is the smallest complete fix for this allocation.

### F2 / COR-ARGON2-TEMPORARIES

- Category and origin: correctness, `correctness-combined p1`
- Confidence: filed **definite**; confirmed **definite**
- Bucket: **would fix**
- Reason: Argon2 0.5.3 gates cleanup of `initial_hash`, `blockhash`, and `blockhash_bytes` on its `zeroize` feature. The dependency declaration enables only the default features, and the Argon2 entry in `Cargo.lock` has no `zeroize` dependency. Enabling the feature is a local, confirmed fix and does not require a design decision.
- Larger proposal not applied: none. This feature change complements F1 because it clears separate internal copies.

### F3 / COR-ADDRSPACE

- Category and origin: correctness, `correctness-combined p2`
- Confidence: filed **possible**; remains **possible**
- Bucket: **would surface**
- Reason: the 4 GiB Argon2 allocation exceeds a 32-bit target's addressable vector size, and `validate_params` has no platform-size guard. The repository's release targets are all 64-bit, but the available project contract does not say whether 32-bit source builds are supported. The fix therefore depends on a platform-support decision rather than confirmation alone.
- Question for the user: Should 32-bit source builds remain supported? If yes, reject an Argon2 workspace whose checked byte size exceeds `isize::MAX`; if no, document and enforce the 64-bit-only requirement.
- Larger proposal not applied: none; nofix mode applies no changes.
```

## RUN-LOG.md

```text
# Pre-PR Review Swarm Run: 20260905-0442-62d866d-4984-combined-astra-high

- Timestamp: 2026-09-05T04:55:38Z
- Repository root: EVAL_ROOT/combined-astra-high/repo
- Commit: 62d866d6a57a24ef6bb329b28a246b44b758ff7a
- Scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)
- Skill arguments: nofix commit
- Coordinator model: gpt-5.6-sol (high)
- Panel: correctness only, explicitly restricted by the user

## Findings report

Reviewed scope: current commit 62d866d (9 touched files, 958 diff lines; correctness only)

Reviewer execution: 1/1 reviewers completed; the user restricted this evaluation panel to correctness, so no other review categories ran.

Reviewer continuation: 1/1 second passes, 1/1 third passes; unavailable: none; capped with new findings: none

Restatement: 3/3 findings restated, 1 attempt(s)

Finding accounting: 3 reviewer findings → 3 reported (0 same-location merges, 0 rejected)

## Correctness

- `F1 / COR-ARGON2-WORKSPACE` — **definite** — `src/format_v2.rs:202` — Wipe the Argon2 workspace before releasing it.

  Argon2 is a memory-hard key-derivation function: while turning the passphrase into an encryption key, it fills a large workspace with password-derived blocks. `derive_key` wraps only the final 32-byte key in `Zeroizing`, then calls Argon2 0.5.3's `hash_password_into` convenience method. That method allocates a normal `Vec<Block>` internally and drops it without clearing its contents. Both saltybox2 encryption and decryption reach this helper, and the default parameters make the abandoned workspace 256 MiB.

  The workspace contains more than disposable bookkeeping. Argon2 computes the returned key by combining the final block from each lane and hashing that value, so anyone who later obtains those released blocks through a memory disclosure or memory capture can reconstruct the encryption key without knowing the passphrase or paying the cost of deriving it again. Wiping the returned key therefore leaves equivalent key material behind.

  Change `derive_key` to allocate the block workspace itself in an explicitly zeroizing container and pass it to `hash_password_into_with_memory`. The container must wipe the allocation whether derivation succeeds or returns an error. Argon2's `zeroize` feature must also be enabled so its public `Block` type implements the cleanup trait required for that container. Enabling the feature by itself is insufficient because `hash_password_into` still creates and releases an ordinary vector.

- `F2 / COR-ARGON2-TEMPORARIES` — **definite** — `Cargo.toml:18` — Enable Argon2's cleanup of internal secret temporaries.

  The new `argon2 = "0.5"` dependency uses Argon2's default feature set, which does not include `zeroize`; the lockfile likewise shows that Argon2 has no dependency on the `zeroize` crate. In Argon2 0.5.3, cleanup of several internal values is compiled only when that feature is active. Without it, the initial hash derived from the password and the final `blockhash` and 1 KiB `blockhash_bytes` buffers remain uncleared when key derivation returns.

  The final two buffers are direct inputs to the last hash that produces the encryption key, so they provide enough material to calculate that key. They are independent copies from both the returned 32-byte key and the large workspace described above; wiping either of those other allocations does not clear these stack temporaries.

  Declare the Argon2 dependency with `features = ["zeroize"]` so the dependency's existing conditional cleanup runs. Keep that feature enabled in addition to explicitly zeroizing the workspace from F1, since the two changes erase different copies of the secret derivation state and neither replaces the other.

- `F3 / COR-ADDRSPACE` — **possible** — `src/format_v2.rs:160` — Reject Argon2 workspace sizes that cannot fit the target address space.

  `validate_params` accepts a memory cost of `4194304` KiB with one pass and one lane on every target because that value is exactly the format's configured upper limit. A hostile 68-byte binary payload can put those values in its 52-byte header and supply only a dummy 16-byte authentication tag. Decryption validates the header and starts key derivation before it authenticates the payload, so no valid passphrase or tag is needed to reach the allocation.

  With those parameters, Argon2 0.5.3 asks `Vec` for 4,194,304 blocks of 1,024 bytes each: exactly 4 GiB. That allocation cannot fit a 32-bit address space and exceeds Rust's maximum vector allocation size, causing a capacity-overflow panic before Argon2 can return an error. The surrounding `map_err` handles only returned Argon2 errors, so it cannot convert this panic into saltybox's structured format or resource error. If 32-bit source builds are supported, a very small malformed encrypted input can therefore terminate the operation.

  Before calling the allocating Argon2 API, use checked multiplication on `params.block_count()` and `Block::SIZE`, then reject any required byte count above the current platform's `isize::MAX` with a structured error. This keeps the format's 4 GiB limit available on address spaces that can support it while rejecting impossible allocations on smaller targets. The existing v1 engine already applies this kind of platform-specific `isize::MAX` guard to an input length. If 32-bit source builds are outside the supported platform set instead, document and enforce that requirement. The release configuration currently names only 64-bit targets, which is why this finding remains conditional rather than claiming that shipped binaries encounter the overflow.

## Confirmation and nofix buckets

# Nofix bucket assignment

No source changes were made. These are the buckets the default fixing mode would use after confirming each finding against the pinned checkout and the on-disk Argon2 0.5.3 source.

### F1 / COR-ARGON2-WORKSPACE

- Category and origin: correctness, `correctness-combined p1`
- Confidence: filed **definite**; confirmed **definite**
- Bucket: **would fix**
- Reason: `derive_key` calls `hash_password_into`, which allocates an ordinary `Vec<Block>`. Argon2 finalization derives the output from the final lane blocks still held by that vector, and neither the vector nor `Block` has active drop-time wiping under the current features. This is a confirmed correctness finding with a complete implementation fix that does not require a public API, format, or policy decision.
- Larger proposal not applied: none. The explicit zeroizing workspace is the smallest complete fix for this allocation.

### F2 / COR-ARGON2-TEMPORARIES

- Category and origin: correctness, `correctness-combined p1`
- Confidence: filed **definite**; confirmed **definite**
- Bucket: **would fix**
- Reason: Argon2 0.5.3 gates cleanup of `initial_hash`, `blockhash`, and `blockhash_bytes` on its `zeroize` feature. The dependency declaration enables only the default features, and the Argon2 entry in `Cargo.lock` has no `zeroize` dependency. Enabling the feature is a local, confirmed fix and does not require a design decision.
- Larger proposal not applied: none. This feature change complements F1 because it clears separate internal copies.

### F3 / COR-ADDRSPACE

- Category and origin: correctness, `correctness-combined p2`
- Confidence: filed **possible**; remains **possible**
- Bucket: **would surface**
- Reason: the 4 GiB Argon2 allocation exceeds a 32-bit target's addressable vector size, and `validate_params` has no platform-size guard. The repository's release targets are all 64-bit, but the available project contract does not say whether 32-bit source builds are supported. The fix therefore depends on a platform-support decision rather than confirmation alone.
- Question for the user: Should 32-bit source builds remain supported? If yes, reject an Argon2 workspace whose checked byte size exceeds `isize::MAX`; if no, document and enforce the 64-bit-only requirement.
- Larger proposal not applied: none; nofix mode applies no changes.
```
