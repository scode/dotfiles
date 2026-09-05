# Finding register

All 89 assessment records are retained below, including invalid, optional, uncertain, and out-of-scope claims. They
derive from 86 original findings; three combined findings were split during assessment. These are assessed claims, not
verbatim raw reviewer transcripts. Original local IDs, model/lens attribution, canonical matches, evidence, and
adjudication are preserved. Source paths refer to each case’s pinned subject in [metadata.json](metadata.json) unless
the evidence explicitly names a base or dependency.

## Accepted case issues

The 22 rows below deduplicate valid findings across lenses within each case. Related commits can expose the same
underlying issue family. Full records follow, including disagreements about these issues.

| Case                                  | Issue                           | Title                                                                            | Valid reporting models                       |
| ------------------------------------- | ------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------- |
| treeward-swapped-fifo                 | TW-DEVICE-NOWAIT                | The new device-wide no-wait specification is too broad                           | Luna high, Sol high                          |
| ferricode-openai-codex-remote-auth    | FC-03                           | Completed public auth calls leave stdin readers alive                            | Luna high, Sol high                          |
| ferricode-openai-codex-remote-auth    | FC-04                           | An empty pasted line aborts a healthy HTTP auth attempt                          | Muse high                                    |
| ferricode-openai-codex-remote-auth    | FC-06                           | Stdin read failure aborts usable HTTP authentication                             | Luna high, Sol high, Muse high               |
| ferricode-openai-codex-remote-auth    | FC-07                           | An accepted incomplete TCP connection delays a ready paste                       | Terra medium                                 |
| ferricode-openai-codex-remote-auth    | FC-08                           | Unavailable callback endpoints only fall back on AddrInUse                       | Luna high                                    |
| ferricode-openai-codex-remote-auth    | FC-09                           | Pasted input grows without a size limit before validation                        | Luna high, Sol high                          |
| dotfiles-scode-chores-initial         | DC-01                           | Dprint alternate-path support contradicts the detection gate                     | Luna high, Sol high, Terra medium, Muse high |
| saltybox-format-dispatch-initial      | dispatch-decrypt-error-doc      | Decrypt documentation overstates authentication-error uniformity                 | Luna high, Muse high                         |
| saltybox-v2-engine-initial            | SVE-001                         | Generator refers to an absent SPEC.md format contract                            | Luna high, Terra medium, Muse high           |
| saltybox-v2-engine-initial            | SVE-002                         | Parallelism rationale names a nonexistent Argon2 feature                         | Terra medium                                 |
| saltybox-v2-engine-initial            | SVE-003                         | Resource-cap comments overstate protection against exhaustion                    | Luna high, Sol high, Terra medium, Muse high |
| saltybox-v2-decrypt-initial           | DECRYPT-README-V2               | README still presents readable saltybox2 files as a future format                | Luna high, Terra medium, Muse high           |
| saltybox-v2-decrypt-initial           | DECRYPT-KDF-RESOURCE-CONTRACT   | Newly accepted v2 headers permit 4 GiB and 64 passes before authentication       | Luna high, Sol high, Terra medium            |
| saltybox-v2-decrypt-initial           | DECRYPT-POST-RENAME-FAILURE     | New failure contract promises unchanged output after an irreversible replacement | Sol high                                     |
| saltybox-v2-write-gate-initial        | SWG-002                         | Default-engine documentation still claims exclusive write selection              | Luna high                                    |
| saltybox-v2-write-gate-initial        | SWG-004                         | README omits the newly available experimental write override                     | Luna high, Terra medium                      |
| saltybox-v2-write-gate-initial        | SWG-005                         | New command-wide failure guarantee excludes a real post-replacement error        | Luna high, Terra medium                      |
| saltybox-v2-flip-initial              | flip-001                        | The test comment overstates removal of the v1 write engine                       | Luna high, Muse high                         |
| stark-parts-pr56-catalog-static-asset | stark56-readme-embedded-catalog | README still describes compile-time catalog inclusion                            | Luna high, Terra medium, Muse high           |
| stark-parts-pr56-catalog-static-asset | stark56-startup-banner          | Startup and permanent failure screens omit the required unofficial-site notice   | Sol high, Terra medium, Muse high            |
| stark-parts-pr57-catalog-only-ci      | SP57-001                        | Catalog-only CI skips tests that depend on the changed catalog                   | Sol high, Muse high                          |

## treeward-swapped-fifo

[Pinned source](https://github.com/scode/treeward/tree/cebefd2c833354352702a2e145526224cd68c648).

### F001: README does not mention the FIFO failure guarantee

**optional / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `TW-README-FIFO`.

**Evidence:**

- README.md:327-331 describes mtime detection and labels concurrent-modification detection best effort; it does not
  claim to enumerate every error.
- SPEC.md:31-32 already records the newly intended FIFO behavior.
- src/checksum.rs:117-133 implements the FIFO fix without changing CLI flags or setup.

**Assessment:** The omission is real and a sentence could help discoverability, but the README statement remains true
and this detail is already documented in SPEC. Treat this as optional documentation expansion, not meaningful
missing-contract coverage.

### F002: The new device-wide no-wait specification is too broad

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-002` · canonical issue `TW-DEVICE-NOWAIT`.

**Evidence:**

- SPEC.md:31-32 newly promises that checksumming a path replaced by a device never waits on the object.
- src/checksum.rs:117-133 opens the object before checking descriptor type; base
  330d48e5e2010a6c3c01a94f8a2e1204b6b5c324 src/checksum.rs:112-128 already used that ordering.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/sg.c#L262 : sg_open at lines 262-310 calls
  scsi_autopm_get_device at line 286 regardless of O_NONBLOCK.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/scsi_pm.c#L204 : lines 204-214 implement
  scsi_autopm_get_device using pm_runtime_get_sync at line 207.
- /usr/src/linux-headers-6.8.0-139/include/linux/pm_runtime.h:405-421 documents and implements synchronous runtime
  resume.

**Assessment:** The possible device premise becomes valid through the driver-open trace: a swapped accessible SCSI
generic node can require synchronous device resume before Treeward rejects it. The new defect is the literal
specification promise, not a runtime regression or a demonstrated indefinite hang. Count minor because only the narrower
documentation overclaim is established. The combined Windows premise is unverified and receives no extra credit; the
original finding remains intact in its raw artifact.

### F003: Windows does not establish the unqualified no-wait promise

**uncertain / minor** — Terra medium · `docs-comments` · local ID `TDC-1` · canonical issue `TW-WINDOWS-NOWAIT`.

**Evidence:**

- src/checksum.rs:138-160 retains a Windows helper without a nonblocking flag; SPEC.md:31-32 does not name a platform.
- dist-workspace.toml:13 lists Linux and macOS release targets only.
- https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-client : CreateFile fails with ERROR_PIPE_BUSY when no
  instance is available; waiting requires a separate WaitNamedPipe call, which Treeward does not make.

**Assessment:** The source difference is real, but absence of an O_NONBLOCK analogue does not establish a Windows
blocking path. The reviewer identifies neither a reachable replacement object nor Windows semantics that violate the
promise, and supported Windows releases are not established. Retain possible concern as uncertain rather than counting
the separately verified Unix device issue as this reviewer's discovery.

### F004: Special-device open can wait before the type check

**valid / minor** — Luna high · `correctness-edge-inputs` · local ID `C1` · canonical issue `TW-DEVICE-NOWAIT`.

**Evidence:**

- src/checksum.rs:117-133 opens before rejecting non-regular descriptors; SPEC.md:31-32 newly promises no waiting for
  device replacements.
- Base 330d48e5e2010a6c3c01a94f8a2e1204b6b5c324 src/checksum.rs:112-128 already opens before metadata rejection, so
  device-open behavior is pre-existing.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/sg.c#L262 : lines 262-310, especially line 286, invoke device
  power management before O_NONBLOCK checks.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/scsi_pm.c#L204 : lines 204-214 invoke pm_runtime_get_sync;
  local /usr/src/linux-headers-6.8.0-139/include/linux/pm_runtime.h:405-421 confirms synchronous resume.
- src/checksum.rs:58-63 and 131-133 ensure special-device reads are never reached.

**Assessment:** Credit the verified mismatch in the newly added device no-wait contract. The report overstates the
evidence when it claims an indefinite hang and mentions device reads: no indefinite duration was established and
non-regular reads are rejected. Those supporting claims are not credited. A reachable example requires an accessible
device node on a device-enabled mount and a swap after classification; that fits the race explicitly named in SPEC.
Narrowing the new SPEC is the justified remedy; a cross-platform handle redesign was not validated.

### F005: O_NONBLOCK does not establish the new device-node guarantee

**valid / minor** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `TW-DEVICE-NOWAIT`.

**Evidence:**

- SPEC.md:31-32 introduces the device no-wait promise; src/checksum.rs:117-133 opens before checking type.
- Base 330d48e5e2010a6c3c01a94f8a2e1204b6b5c324 src/checksum.rs:112-128 already opens potential device nodes, so this is
  not a new runtime regression.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/sg.c#L262 : sg_open lines 262-310 invokes
  scsi_autopm_get_device at 286 before nonblocking handling.
- https://github.com/torvalds/linux/blob/v6.8/drivers/scsi/scsi_pm.c#L204 : lines 204-214 use pm_runtime_get_sync;
  /usr/src/linux-headers-6.8.0-139/include/linux/pm_runtime.h:405-421 specifies synchronous resume.

**Assessment:** The possible claim is verified only as a newly overbroad SPEC promise: a device-open path can
synchronously resume a device before rejection. The local open(2) statement about block-device I/O alone would not
establish this because Treeward never reads those descriptors. Do not count an indefinite hang or a newly introduced
runtime defect. The reviewer correctly includes narrowing SPEC as a remedy. This matches Luna's device finding and is
not a unique Sol issue.

## ferricode-openai-codex-remote-auth

[Pinned source](https://github.com/scode/ferricode/tree/5d794aa16f9a1a975cb94e4bb33e1c4fb94ec0ac).

### F006: Replace detached stdin thread with spawn_blocking

**invalid / minor** — Muse high · `idiomaticity` · local ID `IDIOM-1` · canonical issue `FC-01`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:355-375 explicitly uses an OS thread to avoid Tokio shutdown waiting.
- repo/Cargo.lock:1500-1503 pins Tokio 1.52.3.
- dependency-source/tokio-1.52.3/src/task/blocking.rs:102-118 says started spawn_blocking tasks cannot be aborted and
  runtime shutdown waits indefinitely.
- repo/crates/ferric/src/main.rs:57-66 uses the ordinary tokio::main runtime lifecycle.

**Assessment:** The suggested replacement changes shutdown behavior and can make a successful HTTP login hang at CLI
exit until stdin produces input. Dropping a JoinHandle detaches result ownership, but does not remove the blocking task
from runtime shutdown. The reviewer labels the premise possible; the pinned dependency documentation disproves it.

### F007: Detached-thread comment allegedly describes the wrong resource

**invalid / minor** — Luna high · `docs-comments` · local ID `LUNA-DOCS-001` · canonical issue `FC-02`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:357-359 says a detached OS thread avoids waiting for an uncancellable
  Tokio blocking task, immediately followed by std::thread::spawn.
- dependency-source/tokio-1.52.3/src/task/blocking.rs:106-118 confirms the hypothetical Tokio alternative would delay
  runtime shutdown.

**Assessment:** The comment explicitly describes the chosen OS thread and contrasts it with the avoided Tokio
alternative. It does not claim a Tokio task is created. The finding reverses that meaning, so it is not useful
documentation coverage. More lifecycle documentation could be added, but that does not validate the asserted
contradiction.

### F008: Completed public auth calls leave stdin readers alive

**valid / material** — Luna high · `correctness-data-flow` · local ID `LUNA-CDF-001` · canonical issue `FC-03`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:213-225 exposes authentication publicly and unconditionally starts the
  stdin reader.
- repo/crates/ferricode-openai-codex/src/lib.rs:324-328 returns on HTTP completion without stopping that reader.
- repo/crates/ferricode-openai-codex/src/lib.rs:355-365 detaches the thread and discards send failure after read_line
  completes.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/stdio.rs:412-413 obtains the
  shared stdin lock for read_line.
- repo/crates/ferric/src/main.rs:78,94-121 calls auth once and exits; it does not demonstrate a repeated in-repository
  caller.

**Assessment:** The thread outlives an HTTP-success return. In a long-lived caller, the earlier reader can hold the
shared stdin lock, consume the next auth attempt's callback, and discard it because its receiver was dropped. Subsequent
calls can also accumulate blocked threads. This is a real public-API lifecycle defect with material input-loss/resource
consequences, conditional on reusing the API in a process that remains alive; the shipped one-shot CLI masks it. Simply
joining the thread or switching to spawn_blocking would reintroduce a shutdown hang.

### F009: An empty pasted line aborts a healthy HTTP auth attempt

**valid / minor** — Muse high · `correctness-data-flow` · local ID `F1/listener` · canonical issue `FC-04`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:360-364 returns Some("\n") for an Enter key.
- repo/crates/ferricode-openai-codex/src/lib.rs:856-857 trims that to an empty URL and propagates the parse error.
- repo/crates/ferricode-openai-codex/src/lib.rs:331-336 propagates paste-processing failure out of the listener loop.
- repo/crates/ferricode-openai-codex/src/lib.rs:414-417,868-875 instead recovers HTTP parse/state/missing-code errors.

**Assessment:** Split of original F1: this record assesses its listener-mode claim. The complete raw F1 is retained in
its raw artifact. A harmless extra Enter now cancels a usable browser login and requires restarting the OAuth attempt.
The premise is fully established by source; no token corruption occurs, so the user-recovery cost is minor.

### F010: Paste-only mode does not ignore or retry an empty line

**optional / minor** — Muse high · `correctness-data-flow` · local ID `F1/paste-only` · canonical issue `FC-05`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:308-316 awaits one paste and propagates validation failure.
- repo/SPEC.md:54-62 requires accepting valid callbacks and rejecting others; it does not promise retries or blank-line
  suppression.

**Assessment:** Split of original F1, whose original text covers both listener and paste-only modes. Blank-line
filtering or re-prompting would improve usability, but a command failing after invalid submitted input does not
contradict the specified paste-only contract. There is no previously working alternate channel to preserve in this
branch. The cryptic parse error and retry suggestion are optional usability work.

### F011: Invalid pasted input cancels the concurrent HTTP callback

**valid / minor** — Muse high · `correctness-edge-inputs` · local ID `F1/validation` · canonical issue `FC-04`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:331-336 returns from the whole function for invalid pasted
  URL/state/code.
- repo/crates/ferricode-openai-codex/src/lib.rs:868-875 classifies the equivalent HTTP parse/state/code errors as
  recoverable.
- repo/crates/ferricode-openai-codex/src/lib.rs:338-340 already supports disabling the paste branch after EOF.

**Assessment:** Split of original F1: validation errors and stdin read errors are independently handled at different
propagation sites and need different retry policies. This record covers blank/malformed/wrong-state pasted values; the
read-error portion is separately matched to FC-06. The user can lose a healthy ongoing browser login to one accidental
paste. Valid minor UX regression, not evidence of token theft or persistent data loss.

### F012: Stdin read failure aborts usable HTTP authentication

**valid / material** — Muse high · `correctness-edge-inputs` · local ID `F1/read-error` · canonical issue `FC-06`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:361-364 maps read_line errors to OpenAiCodexError::Io.
- repo/crates/ferricode-openai-codex/src/lib.rs:331-332 propagates that error while the listener exists.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs:384-397,2625-2629
  returns InvalidData when read_line receives invalid UTF-8.

**Assessment:** Split of original F1's explicit stdin-Io example, preserving the original report in full. Invalid UTF-8
or another real read error disables the entire login even though only the optional transport is broken. Unlike EOF, it
prevents authentication with a usable browser callback under that process input configuration. This is also
independently reported by Luna and Sol.

### F013: Paste-only mode offers no retry after malformed input

**optional / minor** — Muse high · `correctness-edge-inputs` · local ID `F2` · canonical issue `FC-05`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:308-316 consumes one paste and returns its validation error.
- repo/SPEC.md:54-62 does not require a retry loop; invalid callbacks must be rejected.

**Assessment:** The observed one-attempt behavior is correct, but automatic retry is an optional UX enhancement rather
than a specified correctness requirement. Ordinary visual terminal wrapping does not itself insert newline bytes, so
that example is not generally established; a genuinely newline-containing paste can still be truncated by read_line. The
same optional issue is present in the paste-only portion of Muse data-flow F1.

### F014: Optional stdin read errors terminate HTTP authentication

**valid / material** — Sol high · `correctness-data-flow` · local ID `CDF-1` · canonical issue `FC-06`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:331-340 treats Err as fatal while treating EOF as disabling only the
  paste branch.
- repo/crates/ferricode-openai-codex/src/lib.rs:361-364 preserves actual read_line errors.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/stdio.rs:98-108,203-207 maps
  EBADF to EOF, disproving the report's closed-descriptor example.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs:384-397,2625-2629
  establishes invalid UTF-8 as a real error trigger.

**Assessment:** The broad finding is valid, but the proposed closed-stdin/Bad-file-descriptor reproducer is wrong for
the inspected standard library. Invalid UTF-8 supplied to stdin proves the actual error path and gives a concrete
configuration where the optional input blocks otherwise viable browser authentication. This repairs the premise rather
than accepting the confident example; the local standard-library version is not pinned by this case.

### F015: An accepted incomplete TCP connection delays a ready paste

**valid / minor** — Terra medium · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `FC-07`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:323-337 runs handle_http_callback_stream inside the selected accept arm.
- repo/crates/ferricode-openai-codex/src/lib.rs:45,397-403 permits a ten-second request-read wait during which the paste
  future is not polled.
- dependency-source/tokio-1.52.3/src/macros/select.rs:61-65 documents randomized branch order at subsequent loop
  iterations.
- repo/crates/ferricode-openai-codex/src/lib.rs:2216-2263 explicitly tests preserving an accepted request when paste EOF
  occurs.

**Assessment:** The concrete ten-second delay is real: a connected peer can withhold request bytes while a valid pasted
callback waits. That is a minor responsiveness defect in the new alternative input path. The stronger
indefinite-starvation claim is not established because each timeout returns to a fairly randomized select. A fix must
preserve accepted callback/token-exchange completion; blindly cancelling arbitrary processing can create a worse race.

### F016: Unavailable callback endpoints only fall back on AddrInUse

**valid / material** — Luna high · `correctness-edge-inputs` · local ID `LUNA-CEI-001` · canonical issue `FC-08`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:247-255 maps only AddrInUse to CallbackPortInUse.
- repo/crates/ferricode-openai-codex/src/lib.rs:230-244 propagates every other bind error before any paste can be read.
- repo/SPEC.md:54-57 says an unavailable port or a remote process must support pasted callbacks.
- repo/README.md:22-26 describes the narrower occupied-port scenario.

**Assessment:** The SPEC promises availability of the independent paste path when the local endpoint is unavailable.
Address-unavailable or permission-denied binding can fail without preventing outbound OAuth requests, but this code
exits before that path is offered. The README's narrower example does not narrow the SPEC contract. Not every OS error
must be ignored blindly; the necessary fix is to distinguish endpoint-only failures from failures that also preclude
paste authentication, or explicitly narrow the contract.

### F017: Invalid UTF-8 on optional stdin aborts browser auth

**valid / material** — Luna high · `correctness-edge-inputs` · local ID `LUNA-CEI-002` · canonical issue `FC-06`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:323-342 propagates the stdin future's Err and drops the listener.
- repo/crates/ferricode-openai-codex/src/lib.rs:355-365 reads stdin into a String and maps errors without isolating the
  optional channel.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs:384-397,2625-2629
  verifies the reported invalid-UTF-8 trigger.

**Assessment:** Unlike Sol's closed-fd example, this report supplies a concrete valid trigger. A binary/invalid-UTF-8
line arrives before the browser callback, read_line errors, and authentication exits despite the healthy listener. Both
transport independence and the previous HTTP-only behavior support the finding.

### F018: Pasted input grows without a size limit before validation

**valid / material** — Luna high · `correctness-edge-inputs` · local ID `LUNA-CEI-003` · canonical issue `FC-09`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:355-365 calls read_line on an unbounded String, including in listener
  mode.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs:2245-2272 appends all
  available data until the delimiter or EOF, with no caller-supplied limit.
- repo/crates/ferricode-openai-codex/src/lib.rs:878-892 shows an existing HTTP-input size guard.
- repo/SPEC.md:61-62 identifies pasted callback URLs as untrusted input.

**Assessment:** The possible finding becomes valid after checking the standard input implementation: a long newline-free
stream grows the buffer without reaching URL validation and can exhaust process memory. This is a material resource
bound defect, even though stdin is a local caller-controlled input and no remote attacker is established. The HTTP
guard's precise overshoot behavior is pre-existing and is not assessed as a new issue here.

### F019: Pasted input is buffered without a size limit

**valid / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `FC-09`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:355-365 calls read_line on an unbounded String, including in listener
  mode.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/mod.rs:2245-2272 appends all
  available data until the delimiter or EOF, with no caller-supplied limit.
- repo/crates/ferricode-openai-codex/src/lib.rs:878-892 shows an existing HTTP-input size guard.
- repo/SPEC.md:61-62 identifies pasted callback URLs as untrusted input.

**Assessment:** Independently matches Luna's unbounded-input finding. The entire line is buffered before URL validation;
local caller control does not eliminate the resource-bound failure. No remote attack capability is claimed.

### F020: Detached stdin readers consume later caller input

**valid / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-2` · canonical issue `FC-03`.

**Evidence:**

- repo/crates/ferricode-openai-codex/src/lib.rs:213-225 exposes authentication publicly and unconditionally starts the
  stdin reader.
- repo/crates/ferricode-openai-codex/src/lib.rs:324-328 returns on HTTP completion without stopping that reader.
- repo/crates/ferricode-openai-codex/src/lib.rs:355-365 detaches the thread and discards send failure after read_line
  completes.
- toolchain-source/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/std/src/io/stdio.rs:412-413 obtains the
  shared stdin lock for read_line.
- repo/crates/ferric/src/main.rs:78,94-121 calls auth once and exits; it does not demonstrate a repeated in-repository
  caller.

**Assessment:** Independently matches Luna's public-API lifecycle finding and correctly distinguishes the exiting CLI
from a long-lived embedding caller. The possible premise is established by the public unrestricted API plus the detached
reader and shared stdin lock; practical impact remains conditional on the process staying alive.

## dotfiles-scode-chores-initial

[Pinned source](https://github.com/scode/dotfiles/tree/6541d924a98a697853ae4ba15df094c837d1b13a).

### F021: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Luna high · `docs-comments` · local ID `DPRINT-CONFIG-DETECTION` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F022: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Luna high · `correctness-data-flow` · local ID `CDF-2` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision. The review called this
possible; the explicit contradiction verifies its premise.

### F023: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Terra medium · `correctness-data-flow` · local ID `CDF-1` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F024: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Luna high · `correctness-edge-inputs` · local ID `F1` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F025: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Muse high · `correctness-edge-inputs` · local ID `F2` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F026: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Sol high · `correctness-data-flow` · local ID `CDF-1` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F027: Dprint alternate-path support contradicts the detection gate

**valid / minor** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `DC-01`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:14-22: detect applicability before discovery and classify unused tools as
  skipped.
- repo/agent-skills/scode-chores/SKILL.md:61-63: root dprint.json detection and explicit absent-file skip.
- repo/agent-skills/scode-chores/SKILL.md:71: explicitly promises to use a different project config path.

**Assessment:** Valid internal contract contradiction. A repository documenting tools/dprint.json is explicitly
contemplated at line 71 but fails the earlier root-file gate. An agent can reconcile this prose intelligently, so a skip
is not inevitable in every execution; the contradictory instruction itself is a real minor defect. No external
filename-support assumption is needed. Resolve the effective path before the skip decision.

### F028: Final discovery diff allegedly remains unconditionally

**invalid / minor** — Luna high · `correctness-data-flow` · local ID `CDF-1` · canonical issue `DC-02`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:16-17: record the summary, then restore the worktree; before trying the next
  chore specifies ordering.
- repo/agent-skills/scode-chores/SKILL.md:27: recreate each approved diff in its own entry.
- repo/agent-skills/jjstack/SKILL.md:186-188: inspect and exclude unrelated changes before a reviewable commit.

**Assessment:** The finding turns an unconditional restore instruction into an if-there-is-a-next-chore condition that
is not written. Step 3 remains a required workflow step for the final chore. The claimed unapproved-diff leak does not
follow from this wording. Saying including the final chore would be optional clarification, not a verified defect.

### F029: Quoted Actions uses values allegedly fail the search

**invalid / minor** — Terra medium · `correctness-data-flow` · local ID `CDF-2` · canonical issue `DC-03`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:90: rg expression is uses:\s*[^./][^@]+@.
- The [^./] class accepts both single and double quote characters; [^@]+ consumes actions/checkout up to @ in either
  quoted example.

**Assessment:** The regex excludes dot and slash, not quote characters. Both quoted examples in the finding match the
prescribed expression. This is a direct regex misreading, independent of YAML behavior.

### F030: Lockfile tracking guard allegedly requires an executable check

**invalid / minor** — Muse high · `correctness-edge-inputs` · local ID `F1` · canonical issue `DC-04`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:39-42: only committed/intentionally committed Cargo.lock is PR-worthy;
  explicitly skip projects that do not commit it.
- repo/agent-skills/scode-chores/SKILL.md:14-15: run updates only for applicable chores.

**Assessment:** This is an agent instruction document. Its explicit skip condition is operational even without a
copyable git command. A literal follower checks the condition before cargo update, not after. Force-adding an ignored
lockfile would violate the existing guard. An example command could be optional guidance, but the reported fall-through
is not supported.

### F031: SHA pin rules allegedly conflict with stable-tag updates

**invalid / minor** — Muse high · `correctness-edge-inputs` · local ID `F3` · canonical issue `DC-05`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:91-92: intentionally pinned SHA/branch refs are ignored unless the user asks
  to change them.
- repo/agent-skills/scode-chores/SKILL.md:93-96: stable-tag lookup/update applies to version-tagged actions.

**Assessment:** The update rule is scoped by the preceding version-tagged action clause. It does not authorize replacing
ignored SHAs with tags. The skill also does not order reporting ignored pins as current. More explicit skipped-pin
reporting is optional, but neither the claimed security downgrade nor the alleged conflicting rules follows from the
text.

### F032: Expand maintenance to SHA-pinned Actions

**out_of_scope / minor** — Sol high · `correctness-data-flow` · local ID `CDF-2` · canonical issue `DC-05`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:91-94: intentional branch/SHA refs are explicitly excluded unless requested;
  only version-tagged actions are checked.
- commit-message.txt: feat: add periodic chores skill, with no promise to update every pin format.

**Assessment:** The observation that ignored SHA pins receive no updates is true and explicitly intentional in this
change. Automatically advancing immutable pins would expand the stated chore boundary. The finding does not establish
that the skill reports excluded pins as current. This is a feature-policy request rather than a correctness defect under
the pinned contract.

### F033: Constrain textual Actions discovery to real YAML uses keys

**optional / minor** — Luna high · `correctness-edge-inputs` · local ID `F2` · canonical issue `DC-06`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:90: unanchored rg search can match # uses: and run: echo examples.
- repo/agent-skills/scode-chores/SKILL.md:83-86: applicability requires an actual workflow step referencing an external
  action.
- repo/agent-skills/scode-chores/SKILL.md:93,98,103-105: update action refs, and inspect the resulting workflow diff.

**Assessment:** The candidate search is broad and a YAML-aware selector would reduce false candidates. But this is a
manual agent procedure, not an automated substitution loop: matching text alone is not declared sufficient for an edit.
The outer detect condition and action-ref wording still require a real action. A definite unexpected rewrite is not
demonstrated, so this is useful optional hardening, not a verified behavior defect.

### F034: Dirty manifests may contaminate a generated lockfile

**uncertain / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-2` · canonical issue `DC-07`.

**Evidence:**

- repo/agent-skills/scode-chores/SKILL.md:31-33: permits in-place discovery if enough state is retained to preserve
  unrelated edits.
- repo/agent-skills/scode-chores/SKILL.md:27-28,47,55-57: recreate approved update and verify; no explicit
  clean-manifest invariant in this skill.
- repo/agent-skills/jjstack/SKILL.md:186-188: path-limited commits can leave unrelated files in the working copy.
- repo/agent-skills/jjstack/SKILL.md:349-357: starting a new stack normally runs jj new trunk() before making the
  change.

**Assessment:** The dependency premise is sound: a generated Cargo.lock depends on Cargo.toml, and committing only the
lockfile does not remove effects of an uncommitted manifest. The finding is more substantive than generic clean-worktree
advice. However, the invoked jjstack workflow normally starts the stack on a clean trunk-based change, preventing the
reported recreation path. No end-to-end execution established a compliant alternative that recreates the lockfile in the
dirty checkout. Retain as uncertain with potential material impact, not a verified material discovery; parent should
independently review the cross-skill boundary.

## saltybox-spec-skeleton-initial

[Pinned source](https://github.com/scode/saltybox/tree/91db5c28acde6a2666bbfc0581c85a1ed6222d32).

### F035: Claim that AGENTS.md requires an explicit relative marker

**invalid / minor** — Luna high · `idiomaticity` · local ID `I1` · canonical issue `saltybox-spec-skeleton-initial-001`.

**Evidence:**

- repo/AGENTS.md:6-8: 'Behavior SPEC.md does not cover is existing-but-unspecified' is followed by the requirement to
  specify touched behavior.
- repo/AGENTS.md:6: In the relative clause 'SPEC.md does not cover', SPEC.md is the subject and Behavior supplies the
  omitted object; English allows omission of an object relative pronoun here.
- repo/SPEC.md:7-9: The same rule is restated as 'Behavior not described here is existing-but-unspecified, not
  nonexistent' and requires specification when that behavior is touched.
- charters/idiomaticity.md:7: The charter excludes subjective stylistic preferences that are not bugs.

**Assessment:** The definite grammatical-error claim is false. This is a standard zero object-relative construction,
like 'the behavior users expect'. Adding 'that' is grammatical too and may suit a reader's preference, but the original
is neither malformed nor materially ambiguous. The report identifies no repository convention requiring the marker. Its
suggested edit is optional prose polish, while its asserted defect is invalid.

## saltybox-move-v1-crypto-initial

[Pinned source](https://github.com/scode/saltybox/tree/0c339b95fc2be4010d3fab7471541d0d691f99d9).

### F036: Require a compatibility alias for the renamed unsupported library module

**invalid / minor** — Terra medium · `correctness-data-flow` · local ID `CDF-1` · canonical issue `MOVE-API-COMPAT`.

**Evidence:**

- repo/src/lib.rs:8 replaces the public secretcrypt module declaration with secretcrypt_v1; old downstream imports would
  indeed fail.
- repo/README.md:93-99 distinguishes persisted-file compatibility from library use, and explicitly says the code is not
  meant to be consumed as a library and may be refactored or changed at will.
- repo/README.md:95 promises old encrypted files remain decryptable; the old src/secretcrypt.rs and new
  src/secretcrypt_v1.rs have identical Git blob OID 47e003194550712acbd31c4d1eec66ff03f3bbd9.
- repo/src/file_ops.rs:8,32,64,110,115 and repo/tests/golden_vectors.rs:90,127 consistently reference the new name with
  unchanged arguments and error handling.
- repo/SPEC.md:3-9 scopes its behavioral specification to the CLI and on-disk formats; this commit changes neither.

**Assessment:** The source-path change is real, but treating it as a bug assumes a library API stability promise that
README.md:98-99 expressly denies. The commit intentionally renames the module. Supported encrypted-file compatibility
and CLI behavior remain unchanged. A compatibility alias and major-version migration are therefore not required by the
applicable contract. Minor denotes the absence of a demonstrated supported-user failure, not a claim that compile
failures are generally minor.

### F037: Require a compatibility alias for the renamed unsupported library module

**invalid / minor** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `MOVE-API-COMPAT`.

**Evidence:**

- repo/src/lib.rs:8 replaces the public secretcrypt module declaration with secretcrypt_v1; old downstream imports would
  indeed fail.
- repo/README.md:93-99 distinguishes persisted-file compatibility from library use, and explicitly says the code is not
  meant to be consumed as a library and may be refactored or changed at will.
- repo/README.md:95 promises old encrypted files remain decryptable; the old src/secretcrypt.rs and new
  src/secretcrypt_v1.rs have identical Git blob OID 47e003194550712acbd31c4d1eec66ff03f3bbd9.
- repo/src/file_ops.rs:8,32,64,110,115 and repo/tests/golden_vectors.rs:90,127 consistently reference the new name with
  unchanged arguments and error handling.
- repo/SPEC.md:3-9 scopes its behavioral specification to the CLI and on-disk formats; this commit changes neither.

**Assessment:** The source-path change is real, but treating it as a bug assumes a library API stability promise that
README.md:98-99 expressly denies. The commit intentionally renames the module. Supported encrypted-file compatibility
and CLI behavior remain unchanged. A compatibility alias and major-version migration are therefore not required by the
applicable contract. Minor denotes the absence of a demonstrated supported-user failure, not a claim that compile
failures are generally minor.

## saltybox-format-dispatch-initial

[Pinned source](https://github.com/scode/saltybox/tree/a4afb8c9af97db2d4f0a3a815f0170839814d622).

### F038: Reduce armor-marker constants to crate visibility

**optional / minor** — Luna high · `idiomaticity` · local ID `I-001` · canonical issue
`dispatch-public-magic-constants`.

**Evidence:**

- src/varmor.rs:13-16 exports MAGIC_PREFIX and V1_MAGIC as public constants.
- src/format.rs:59-60 and 116 use the constants within the same crate; src/lib.rs:11 publicly exposes varmor.
- The pinned src tree contains no pub(crate) convention. src/format.rs:31-33 also intentionally exposes
  FormatEngine::magic, which returns the marker value.

**Assessment:** Crate visibility would suffice for the new callers and avoid two additional public names, so this is
reasonable API housekeeping. The report does not establish an actual repository convention violation or behavioral
defect. V1 armor markers are already observable wire-format identifiers and must stay compatible regardless of Rust
visibility. Treat the suggestion as optional, not valid idiomaticity coverage.

### F039: Defer the dispatch abstraction until a second engine exists

**invalid / minor** — Luna high · `ai-slop` · local ID `AI-SLOP-001` · canonical issue `dispatch-premature-abstraction`.

**Evidence:**

- commit-message.txt explicitly states: refactor: route CLI through format-engine dispatch.
- src/format.rs:3-13 explains version coexistence and keeping old implementations independent; lines 26-30 explain the
  split diagnostic layers.
- lore/2026-07-01-saltybox2.md:217-237 (Step 3) explicitly requires a dispatch trait with v1 as the only
  registered/default engine and unchanged v1 implementation.
- src/format.rs:98-129 reproduces the three existing varmor::unwrap diagnostic branches while leaving the frozen v1
  armoring path in place.

**Assessment:** The report correctly observes a one-engine registry and duplicated diagnostics, but its recommendation
reverses the exact planned staging requirement. This abstraction has a documented purpose in the current refactor; a
second implementation is deliberately a separate reviewable step. No broken behavior, unused dispatch path, or
inconsistency is demonstrated. Preserve the original aggregate finding as one record because its components all support
the same rejected recommendation.

### F040: Decrypt documentation overstates authentication-error uniformity

**valid / minor** — Luna high · `docs-comments` · local ID `DC-1` · canonical issue `dispatch-decrypt-error-doc`.

**Evidence:**

- src/format.rs:45-49 says bad passphrases and corrupted input are both reported as authentication failures.
- src/format.rs:72-73 delegates decrypt to secretcrypt_v1::decrypt without changing error kinds.
- src/secretcrypt_v1.rs:136-217 returns TruncatedInput, BinaryFormat, or TrailingData for malformed binary input before
  authentication; lines 222-228 map secretbox failure to AuthenticationFailed.
- src/varmor.rs:34-44 accepts correctly marked base64 independent of binary-payload structure; for example, saltybox1:
  yields an empty payload, which fails with TruncatedInput at secretcrypt_v1.rs:136-141. This path is established from
  source, without running code.

**Assessment:** The new public trait documentation is broader than its existing implementation. The claim should be
limited to failures at authentication, while retaining documented structural parse errors. This is a real minor
documentation defect, not a runtime regression or a routine missing-docstring request.

### F041: Decrypt documentation overstates authentication-error uniformity

**valid / minor** — Muse high · `docs-comments` · local ID `F1` · canonical issue `dispatch-decrypt-error-doc`.

**Evidence:**

- src/format.rs:45-49 says bad passphrases and corrupted input are both reported as authentication failures.
- src/format.rs:72-73 delegates decrypt to secretcrypt_v1::decrypt without changing error kinds.
- src/secretcrypt_v1.rs:136-217 returns TruncatedInput, BinaryFormat, or TrailingData for malformed binary input before
  authentication; lines 222-228 map secretbox failure to AuthenticationFailed.
- src/varmor.rs:34-44 accepts correctly marked base64 independent of binary-payload structure; for example, saltybox1:
  yields an empty payload, which fails with TruncatedInput at secretcrypt_v1.rs:136-141. This path is established from
  source, without running code.

**Assessment:** The new public trait documentation is broader than its existing implementation. The claim should be
limited to failures at authentication, while retaining documented structural parse errors. This is a real minor
documentation defect, not a runtime regression or a routine missing-docstring request. Muse labeled its finding
possible; source verification establishes the premise. Its ancillary reference to format.rs:250-305 is incorrect (the
file has 234 lines); the actual taxonomy tests are at 173-233, which does not undermine the finding anchored at 45-49.

### F042: Specify baseline CLI and file-format behavior during the dispatch refactor

**invalid / minor** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue
`dispatch-baseline-spec-coverage`.

**Evidence:**

- AGENTS.md:3-8 requires SPEC updates when user-visible behavior changes and says touched unspecified behavior must be
  specified; SPEC.md:7-9 repeats the latter wording.
- SPEC.md:13-17 deliberately leaves the existing commands and saltybox1 format unspecified.
- lore/2026-07-01-saltybox2.md:217-237 defines this exact dispatch step as a zero-functional-change refactor, with v1
  the only engine, byte-identical errors, and explicit acceptance that SPEC.md needs no update.
- src/format.rs:58-74 delegates all v1 crypto/armor operations unchanged; src/format.rs:98-129 matches the existing
  src/varmor.rs:34-73 classification and messages; scope.diff preserves operation ordering and error contexts in
  file_ops.rs.

**Assessment:** There is a real wording tension: the general rule uses touches, which could be read to include any
implementation refactor. In this case the specific pinned plan explicitly stages baseline specification later and says
this step needs no SPEC update. The diff changes implementation routing without changing accepted bytes, output format,
or diagnostics. Reading touches as changes to user-visible behavior is coherent with that explicit scope and with SPEC
being a behavior document rather than implementation documentation. Reject the claimed completion violation for this
deliberately staged refactor; do not turn a missing pre-existing baseline into a new runtime defect.

## saltybox-v2-engine-initial

[Pinned source](https://github.com/scode/saltybox/tree/62d866d6a57a24ef6bb329b28a246b44b758ff7a).

### F043: Generator refers to an absent SPEC.md format contract

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `SVE-001`.

**Evidence:**

- testdata/generate-golden-vectors-v2.py:10-13 names SPEC.md as the format authority.
- SPEC.md:15-17 contains only a not-yet-specified v1 placeholder; no v2 specification exists.
- src/format_v2.rs:1-23 contains the current byte-layout description.
- lore/2026-07-01-saltybox2.md:271-273 explicitly leaves SPEC.md unchanged for this step; lines 287-290 schedule the
  format specification for step 5.

**Assessment:** The new reference is broken and merits correction even though omission of the normative v2 specification
is deliberate at this unwired stage. Accept the report's alternative to name the existing temporary format description;
do not require moving the future specification work into this commit. The frozen-vector policy can stand independently
of this bad reference.

### F044: Generator refers to an absent SPEC.md format contract

**valid / minor** — Terra medium · `docs-comments` · local ID `DOC-1` · canonical issue `SVE-001`.

**Evidence:**

- testdata/generate-golden-vectors-v2.py:10-13 names SPEC.md as the format authority.
- SPEC.md:15-17 contains only a not-yet-specified v1 placeholder; no v2 specification exists.
- src/format_v2.rs:1-23 contains the current byte-layout description.
- lore/2026-07-01-saltybox2.md:271-273 explicitly leaves SPEC.md unchanged for this step; lines 287-290 schedule the
  format specification for step 5.

**Assessment:** The new reference is broken and merits correction even though omission of the normative v2 specification
is deliberate at this unwired stage. Accept the report's alternative to name the existing temporary format description;
do not require moving the future specification work into this commit. The frozen-vector policy can stand independently
of this bad reference.

### F045: Generator refers to an absent SPEC.md format contract

**valid / minor** — Muse high · `docs-comments` · local ID `F1` · canonical issue `SVE-001`.

**Evidence:**

- testdata/generate-golden-vectors-v2.py:10-13 names SPEC.md as the format authority.
- SPEC.md:15-17 contains only a not-yet-specified v1 placeholder; no v2 specification exists.
- src/format_v2.rs:1-23 contains the current byte-layout description.
- lore/2026-07-01-saltybox2.md:271-273 explicitly leaves SPEC.md unchanged for this step; lines 287-290 schedule the
  format specification for step 5.

**Assessment:** The new reference is broken and merits correction even though omission of the normative v2 specification
is deliberate at this unwired stage. Accept the report's alternative to name the existing temporary format description;
do not require moving the future specification work into this commit. The frozen-vector policy can stand independently
of this bad reference.

### F046: Parallelism rationale names a nonexistent Argon2 feature

**valid / minor** — Terra medium · `docs-comments` · local ID `DOC-2` · canonical issue `SVE-002`.

**Evidence:**

- src/format_v2.rs:64-66 claims a rayon-based parallel feature.
- Cargo.lock:72-84 pins argon2 0.5.3.
- dependency-source/argon2-0.5.3/Cargo.toml:68-80 lists the dependency features, with no parallel feature or rayon
  dependency.
- dependency-source/argon2-0.5.3/src/lib.rs:325-334 processes lanes in an ordinary serial loop.

**Assessment:** Verified the decisive dependency premise locally. The unavailable feature makes the new rationale
inaccurate even though the same mistake appears in the historical plan. The finding does not establish a runtime
cryptography defect or a need to change p=1; correct the rationale. Historical removal timing and claims that increasing
p necessarily adds work were not needed for this verdict.

### F047: Resource-cap comments overstate protection against exhaustion

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-002` · canonical issue `SVE-003`.

**Evidence:**

- src/format_v2.rs:16-19,69-81,125-130 describes the caps as preventing hostile memory/CPU bombs.
- src/format_v2.rs:131-179 accepts m=4194304, t=64, p=1 (or p=8).
- src/format_v2.rs:308-352 accepts a 52-byte header plus 16-byte tag and derives before authentication at lines 354-370.
- dependency-source/argon2-0.5.3/src/lib.rs:229-231 uses vec![Block::default(); block_count()] for the Argon2 workspace;
  src/params.rs:220-238 computes its block count.
- lore/2026-07-01-saltybox2.md:110-115 prescribes exactly these caps; step 4 requires an unwired engine.
- src/format.rs:90,99-101 retains v1 registration and write defaults; public format_v2::decrypt remains directly
  callable.

**Assessment:** The possible premise is verified: accepted unauthenticated input requests 4 GiB through an infallible
allocation and up to 64 passes. The comments should promise finite bounds and describe residual risk rather than
protection from resource bombs. Grade the actual documentation correction as minor; this does not establish that the
deliberately prescribed numeric policy must change. No OOM or CPU-duration measurement was performed.

### F048: Accepted KDF memory cap permits process-wide resource exhaustion

**valid / minor** — Luna high · `correctness-edge-inputs` · local ID `CEE-1` · canonical issue `SVE-003`.

**Evidence:**

- src/format_v2.rs:16-19,69-81,125-130 describes the caps as preventing hostile memory/CPU bombs.
- src/format_v2.rs:131-179 accepts m=4194304, t=64, p=1 (or p=8).
- src/format_v2.rs:308-352 accepts a 52-byte header plus 16-byte tag and derives before authentication at lines 354-370.
- dependency-source/argon2-0.5.3/src/lib.rs:229-231 uses vec![Block::default(); block_count()] for the Argon2 workspace;
  src/params.rs:220-238 computes its block count.
- lore/2026-07-01-saltybox2.md:110-115 prescribes exactly these caps; step 4 requires an unwired engine.
- src/format.rs:90,99-101 retains v1 registration and write defaults; public format_v2::decrypt remains directly
  callable.

**Assessment:** The accepted 4 GiB allocation and pre-authentication work are verified, and this report explicitly
identifies the same false assurance in the new comments as Luna DOC-002. Accept that discovery as valid/minor regardless
of whether the preferred remedy changes the code or the documentation. The numeric caps are prescribed by the pinned
target design and the CLI remains unwired; lower limits, a combined-work budget, or fallible allocation remain optional
hardening, not a verified required behavioral correction. A Result API alone does not promise recovery from every Rust
allocation failure. Initially the coordinator graded this report optional/material based on its main proposed
resource-policy remedy. The parent corrected that calibration so the reported false assurance receives equal discovery
credit across lenses; no extra finding was split out. Calling the underlying policy itself a material defect would
upgrade one recovery for every model and add no unique Sol issue.

### F049: Accepted header values permit 4 GiB and 64 Argon2 passes

**valid / minor** — Terra medium · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `SVE-003`.

**Evidence:**

- src/format_v2.rs:16-19,69-81,125-130 describes the caps as preventing hostile memory/CPU bombs.
- src/format_v2.rs:131-179 accepts m=4194304, t=64, p=1 (or p=8).
- src/format_v2.rs:308-352 accepts a 52-byte header plus 16-byte tag and derives before authentication at lines 354-370.
- dependency-source/argon2-0.5.3/src/lib.rs:229-231 uses vec![Block::default(); block_count()] for the Argon2 workspace;
  src/params.rs:220-238 computes its block count.
- lore/2026-07-01-saltybox2.md:110-115 prescribes exactly these caps; step 4 requires an unwired engine.
- src/format.rs:90,99-101 retains v1 registration and write defaults; public format_v2::decrypt remains directly
  callable.

**Assessment:** The accepted 4 GiB allocation and pre-authentication work are verified, and this report explicitly
identifies the same false assurance in the new comments as Luna DOC-002. Accept that discovery as valid/minor regardless
of whether the preferred remedy changes the code or the documentation. The numeric caps are prescribed by the pinned
target design and the CLI remains unwired; lower limits, a combined-work budget, or fallible allocation remain optional
hardening, not a verified required behavioral correction. A Result API alone does not promise recovery from every Rust
allocation failure. Initially the coordinator graded this report optional/material based on its main proposed
resource-policy remedy. The parent corrected that calibration so the reported false assurance receives equal discovery
credit across lenses; no extra finding was split out. Calling the underlying policy itself a material defect would
upgrade one recovery for every model and add no unique Sol issue.

### F050: At-cap headers permit large allocation and CPU work

**valid / minor** — Muse high · `correctness-edge-inputs` · local ID `F1` · canonical issue `SVE-003`.

**Evidence:**

- src/format_v2.rs:16-19,69-81,125-130 describes the caps as preventing hostile memory/CPU bombs.
- src/format_v2.rs:131-179 accepts m=4194304, t=64, p=1 (or p=8).
- src/format_v2.rs:308-352 accepts a 52-byte header plus 16-byte tag and derives before authentication at lines 354-370.
- dependency-source/argon2-0.5.3/src/lib.rs:229-231 uses vec![Block::default(); block_count()] for the Argon2 workspace;
  src/params.rs:220-238 computes its block count.
- lore/2026-07-01-saltybox2.md:110-115 prescribes exactly these caps; step 4 requires an unwired engine.
- src/format.rs:90,99-101 retains v1 registration and write defaults; public format_v2::decrypt remains directly
  callable.

**Assessment:** The accepted 4 GiB allocation and pre-authentication work are verified, and this report explicitly
identifies the same false assurance in the new comments as Luna DOC-002. Accept that discovery as valid/minor regardless
of whether the preferred remedy changes the code or the documentation. The numeric caps are prescribed by the pinned
target design and the CLI remains unwired; lower limits, a combined-work budget, or fallible allocation remain optional
hardening, not a verified required behavioral correction. A Result API alone does not promise recovery from every Rust
allocation failure. Initially the coordinator graded this report optional/material based on its main proposed
resource-policy remedy. The parent corrected that calibration so the reported false assurance receives equal discovery
credit across lenses; no extra finding was split out. Calling the underlying policy itself a material defect would
upgrade one recovery for every model and add no unique Sol issue. Muse explicitly offers documenting residual risk as
its minimum remedy. Several reported line numbers are wrong; the evidence above provides corrected anchors.

### F051: Accepted KDF maxima permit a tiny input to exhaust the machine

**valid / minor** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue `SVE-003`.

**Evidence:**

- src/format_v2.rs:16-19,69-81,125-130 describes the caps as preventing hostile memory/CPU bombs.
- src/format_v2.rs:131-179 accepts m=4194304, t=64, p=1 (or p=8).
- src/format_v2.rs:308-352 accepts a 52-byte header plus 16-byte tag and derives before authentication at lines 354-370.
- dependency-source/argon2-0.5.3/src/lib.rs:229-231 uses vec![Block::default(); block_count()] for the Argon2 workspace;
  src/params.rs:220-238 computes its block count.
- lore/2026-07-01-saltybox2.md:110-115 prescribes exactly these caps; step 4 requires an unwired engine.
- src/format.rs:90,99-101 retains v1 registration and write defaults; public format_v2::decrypt remains directly
  callable.

**Assessment:** The accepted 4 GiB allocation and pre-authentication work are verified, and this report explicitly
identifies the same false assurance in the new comments as Luna DOC-002. Accept that discovery as valid/minor regardless
of whether the preferred remedy changes the code or the documentation. The numeric caps are prescribed by the pinned
target design and the CLI remains unwired; lower limits, a combined-work budget, or fallible allocation remain optional
hardening, not a verified required behavioral correction. A Result API alone does not promise recovery from every Rust
allocation failure. Initially the coordinator graded this report optional/material based on its main proposed
resource-policy remedy. The parent corrected that calibration so the reported false assurance receives equal discovery
credit across lenses; no extra finding was split out. Calling the underlying policy itself a material defect would
upgrade one recovery for every model and add no unique Sol issue.

### F052: Argon2 errors retain display text but lack typed source chains

**optional / minor** — Luna high · `idiomaticity` · local ID `I1` · canonical issue `SVE-004`.

**Evidence:**

- src/format_v2.rs:193-210 renders Argon2 errors in SaltyboxError messages.
- src/secretcrypt_v1.rs:41-63 retains typed scrypt source errors.
- src/error.rs:91-97 requires StdError for with_kind_and_source.
- Cargo.toml:19 requests argon2 without std.
- dependency-source/argon2-0.5.3/Cargo.toml:68-80 excludes std from default features;
  dependency-source/argon2-0.5.3/src/error.rs:123-124 gates the StdError implementation on std.

**Assessment:** The absence of a typed source is factual, but this is not the same available API situation as scrypt.
The suggested replacement alone would not compile under the selected features; enabling std or adding a wrapper would be
an additional design change. Messages already preserve the cause, and no consumer requiring typed Argon2 sources is
established. Treat that enhancement as optional, not a swallowed-error or definite idiom defect.

### F053: Short constant and type comments are alleged to be vacuous slop

**invalid / minor** — Luna high · `ai-slop` · local ID `F1` · canonical issue `SVE-005`.

**Evidence:**

- src/format_v2.rs:38-54 contains short field-size documentation.
- src/secretcrypt_v1.rs:21-28 already uses the same salt, nonce, and key-length comments.
- src/format_v2.rs:83-88 follows its short type summary with substantive armor/authentication rationale.
- charters/ai-slop.md explicitly excludes patterns consistent with the surrounding codebase.

**Assessment:** This flags an established neighboring convention and partially labels substantive type documentation as
vacuous. The charter excludes consistent repository patterns; removing short comments would be a subjective cleanup, not
an actionable slop finding in this change.

### F054: Use expect explanations for proven four-byte parameter conversions

**optional / minor** — Muse high · `idiomaticity` · local ID `IDIOM-1` · canonical issue `SVE-006`.

**Evidence:**

- src/format_v2.rs:323 verifies the full 52-byte header.
- src/format_v2.rs:335-337 converts fixed four-byte parameter slices with unwrap.
- src/format_v2.rs:332-340 and src/secretcrypt_v1.rs:143-169 use nearby expect messages.

**Assessment:** The local consistency observation is accurate, but all conversions are provably infallible. Extra
invariant text is a reasonable readability preference; the report itself acknowledges no behavior change. It does not
meet the charter's exclusion of subjective style preferences as a definite defect.

## saltybox-v2-decrypt-initial

[Pinned source](https://github.com/scode/saltybox/tree/78beaf7487e99a8f809a6aa308713af81b8fbd23).

### F055: README still presents readable saltybox2 files as a future format

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `DECRYPT-README-V2`.

**Evidence:**

- README.md:67-68 describes scrypt/NaCl without distinguishing accepted input formats; README.md:113-126 documents v1
  output and calls saltybox2 a future version.
- src/format.rs:79-92 registers V2Engine for reads while leaving V1Engine as the write default.
- src/file_ops.rs:62-66 and 109-120 route decrypt/update validation through read dispatch and update writes through the
  unchanged default.
- SPEC.md:26-28 newly promises both saltybox1 and saltybox2 input support.

**Assessment:** The README-drift carve-out applies to the newly supported user-visible input format. The v1 section
remains correct about current output, but its future-v2 statement and lack of read compatibility guidance are stale. The
suggested full v2 format section is more extensive than needed; a short read/write compatibility note and corrected
future wording address the verified drift.

### F056: README still presents readable saltybox2 files as a future format

**valid / minor** — Terra medium · `docs-comments` · local ID `DOCS-1` · canonical issue `DECRYPT-README-V2`.

**Evidence:**

- README.md:67-68 describes scrypt/NaCl without distinguishing accepted input formats; README.md:113-126 documents v1
  output and calls saltybox2 a future version.
- src/format.rs:79-92 registers V2Engine for reads while leaving V1Engine as the write default.
- src/file_ops.rs:62-66 and 109-120 route decrypt/update validation through read dispatch and update writes through the
  unchanged default.
- SPEC.md:26-28 newly promises both saltybox1 and saltybox2 input support.

**Assessment:** The README-drift carve-out applies to the newly supported user-visible input format. The v1 section
remains correct about current output, but its future-v2 statement and lack of read compatibility guidance are stale. The
generic algorithm summary and stale future-version wording are facets of one user-facing format-documentation issue, not
separate defects.

### F057: README still presents readable saltybox2 files as a future format

**valid / minor** — Muse high · `docs-comments` · local ID `F1` · canonical issue `DECRYPT-README-V2`.

**Evidence:**

- README.md:67-68 describes scrypt/NaCl without distinguishing accepted input formats; README.md:113-126 documents v1
  output and calls saltybox2 a future version.
- src/format.rs:79-92 registers V2Engine for reads while leaving V1Engine as the write default.
- src/file_ops.rs:62-66 and 109-120 route decrypt/update validation through read dispatch and update writes through the
  unchanged default.
- SPEC.md:26-28 newly promises both saltybox1 and saltybox2 input support.

**Assessment:** The README-drift carve-out applies to the newly supported user-visible input format. The v1 section
remains correct about current output, but its future-v2 statement and lack of read compatibility guidance are stale.
Promoted from possible after verifying the read capability is unconditional and already present at this pinned commit.
No evidence authorizes deferring a small README compatibility correction; a complete format-section rewrite is
unnecessary.

### F058: Newly accepted v2 headers permit 4 GiB and 64 passes before authentication

**valid / material** — Luna high · `correctness-data-flow` · local ID `CDF-001` · canonical issue
`DECRYPT-KDF-RESOURCE-CONTRACT`.

**Evidence:**

- SPEC.md:95-103 newly claims header validation prevents large hostile memory/CPU consumption while accepting m=4194304
  KiB, t=64, p=1..8.
- src/format.rs:82 newly registers V2Engine; the base revision has only V1Engine. src/file_ops.rs:62-66 and 109-114
  expose it to both CLI read paths.
- src/format_v2.rs:315-369 accepts a complete 52-byte header plus 16 dummy sealed bytes with those in-range parameters.
- src/format_v2.rs:201-212 allocates the Argon2 work buffer; :369-387 derives before checking the authentication tag.
- Cargo.lock:72-75 pins argon2 0.5.3. Local argon2-0.5.3/src/params.rs:223-237 computes 4194304 blocks for these m and p
  values; src/block.rs:51-59 defines a 1024-byte Block. Total is 4294967296 bytes (4 GiB).
- Local argon2-0.5.3/src/lib.rs:298 and :326 use t_cost as the iteration count. The allowed example requests 64 passes.

**Assessment:** A bounded KDF can still be very expensive. This change newly exposes the work through CLI dispatch and
adds a concrete contradictory hostile-resource guarantee. The verdict is anchored in that guarantee and the verified
workload, not an assumed universal safe cap. Lowering accepted limits is a compatibility policy choice; correcting the
guarantee or adding an explicit resource budget can address it. Promoted from possible because local pinned dependency
source proves the buffer size and work count. Process termination and its exact exit status are host-dependent; the
finding does not require every host to crash.

### F059: Newly accepted v2 headers permit 4 GiB and 64 passes before authentication

**valid / material** — Terra medium · `correctness-data-flow` · local ID `CDF-001` · canonical issue
`DECRYPT-KDF-RESOURCE-CONTRACT`.

**Evidence:**

- SPEC.md:95-103 newly claims header validation prevents large hostile memory/CPU consumption while accepting m=4194304
  KiB, t=64, p=1..8.
- src/format.rs:82 newly registers V2Engine; the base revision has only V1Engine. src/file_ops.rs:62-66 and 109-114
  expose it to both CLI read paths.
- src/format_v2.rs:315-369 accepts a complete 52-byte header plus 16 dummy sealed bytes with those in-range parameters.
- src/format_v2.rs:201-212 allocates the Argon2 work buffer; :369-387 derives before checking the authentication tag.
- Cargo.lock:72-75 pins argon2 0.5.3. Local argon2-0.5.3/src/params.rs:223-237 computes 4194304 blocks for these m and p
  values; src/block.rs:51-59 defines a 1024-byte Block. Total is 4294967296 bytes (4 GiB).
- Local argon2-0.5.3/src/lib.rs:298 and :326 use t_cost as the iteration count. The allowed example requests 64 passes.

**Assessment:** A bounded KDF can still be very expensive. This change newly exposes the work through CLI dispatch and
adds a concrete contradictory hostile-resource guarantee. The verdict is anchored in that guarantee and the verified
workload, not an assumed universal safe cap. Lowering accepted limits is a compatibility policy choice; correcting the
guarantee or adding an explicit resource budget can address it. The report incorrectly implies a format error is
promised for this in-range example; SPEC explicitly accepts it. That overstatement does not invalidate the demonstrated
resource-contract mismatch.

### F060: Newly accepted v2 headers permit 4 GiB and 64 passes before authentication

**valid / material** — Sol high · `correctness-data-flow` · local ID `CDF-1` · canonical issue
`DECRYPT-KDF-RESOURCE-CONTRACT`.

**Evidence:**

- SPEC.md:95-103 newly claims header validation prevents large hostile memory/CPU consumption while accepting m=4194304
  KiB, t=64, p=1..8.
- src/format.rs:82 newly registers V2Engine; the base revision has only V1Engine. src/file_ops.rs:62-66 and 109-114
  expose it to both CLI read paths.
- src/format_v2.rs:315-369 accepts a complete 52-byte header plus 16 dummy sealed bytes with those in-range parameters.
- src/format_v2.rs:201-212 allocates the Argon2 work buffer; :369-387 derives before checking the authentication tag.
- Cargo.lock:72-75 pins argon2 0.5.3. Local argon2-0.5.3/src/params.rs:223-237 computes 4194304 blocks for these m and p
  values; src/block.rs:51-59 defines a 1024-byte Block. Total is 4294967296 bytes (4 GiB).
- Local argon2-0.5.3/src/lib.rs:298 and :326 use t_cost as the iteration count. The allowed example requests 64 passes.

**Assessment:** A bounded KDF can still be very expensive. This change newly exposes the work through CLI dispatch and
adds a concrete contradictory hostile-resource guarantee. The verdict is anchored in that guarantee and the verified
workload, not an assumed universal safe cap. Lowering accepted limits is a compatibility policy choice; correcting the
guarantee or adding an explicit resource budget can address it. The report correctly identifies the
header-to-allocation-to-authentication order and offers a compatibility-aware opt-in remedy.

### F061: Newly accepted v2 headers permit 4 GiB and 64 passes before authentication

**valid / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue
`DECRYPT-KDF-RESOURCE-CONTRACT`.

**Evidence:**

- SPEC.md:95-103 newly claims header validation prevents large hostile memory/CPU consumption while accepting m=4194304
  KiB, t=64, p=1..8.
- src/format.rs:82 newly registers V2Engine; the base revision has only V1Engine. src/file_ops.rs:62-66 and 109-114
  expose it to both CLI read paths.
- src/format_v2.rs:315-369 accepts a complete 52-byte header plus 16 dummy sealed bytes with those in-range parameters.
- src/format_v2.rs:201-212 allocates the Argon2 work buffer; :369-387 derives before checking the authentication tag.
- Cargo.lock:72-75 pins argon2 0.5.3. Local argon2-0.5.3/src/params.rs:223-237 computes 4194304 blocks for these m and p
  values; src/block.rs:51-59 defines a 1024-byte Block. Total is 4294967296 bytes (4 GiB).
- Local argon2-0.5.3/src/lib.rs:298 and :326 use t_cost as the iteration count. The allowed example requests 64 passes.

**Assessment:** A bounded KDF can still be very expensive. This change newly exposes the work through CLI dispatch and
adds a concrete contradictory hostile-resource guarantee. The verdict is anchored in that guarantee and the verified
workload, not an assumed universal safe cap. Lowering accepted limits is a compatibility policy choice; correcting the
guarantee or adding an explicit resource budget can address it. The report correctly notes that fallible allocation
alone does not address CPU work or memory pressure after allocation succeeds.

### F062: New failure contract promises unchanged output after an irreversible replacement

**valid / material** — Sol high · `correctness-data-flow` · local ID `CDF-2` · canonical issue
`DECRYPT-POST-RENAME-FAILURE`.

**Evidence:**

- SPEC.md:30-32 adds the unconditional promise that an existing decrypt output is unchanged on any failure; the base
  SPEC has no decrypt contract.
- src/file_ops.rs:248-260 persists the new temporary file over the output before :265-272 performs fallible directory
  sync.
- src/file_ops.rs:67-68 propagates that post-replacement failure to decrypt's caller; no old-output restoration follows.
- Cargo.lock:704-707 pins tempfile 3.27.0; local tempfile-3.27.0/src/file/mod.rs:767-770 calls TempPath::persist, whose
  :204-212 persists with overwrite=true before returning success.

**Assessment:** The write implementation predates this diff, but the unconditional guarantee is newly added in the
scoped SPEC. A fallible directory sync after successful replacement can return failure with new complete plaintext
already installed. This matters to recovery and confidentiality assumptions. Describing the actual commit boundary is
sufficient; guaranteed rollback is not assumed feasible.

### F063: New failure contract promises unchanged output after an irreversible replacement

**valid / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-2` · canonical issue
`DECRYPT-POST-RENAME-FAILURE`.

**Evidence:**

- SPEC.md:30-32 adds the unconditional promise that an existing decrypt output is unchanged on any failure; the base
  SPEC has no decrypt contract.
- src/file_ops.rs:248-260 persists the new temporary file over the output before :265-272 performs fallible directory
  sync.
- src/file_ops.rs:67-68 propagates that post-replacement failure to decrypt's caller; no old-output restoration follows.
- Cargo.lock:704-707 pins tempfile 3.27.0; local tempfile-3.27.0/src/file/mod.rs:767-770 calls TempPath::persist, whose
  :204-212 persists with overwrite=true before returning success.

**Assessment:** The write implementation predates this diff, but the unconditional guarantee is newly added in the
scoped SPEC. A fallible directory sync after successful replacement can return failure with new complete plaintext
already installed. This matters to recovery and confidentiality assumptions. Describing the actual commit boundary is
sufficient; guaranteed rollback is not assumed feasible.

## saltybox-v2-write-gate-initial

[Pinned source](https://github.com/scode/saltybox/tree/ef101f7fc5b03f845041eea685d17baa3019c881).

### F064: Rename the arbitrary-value v2 override test helper

**optional / minor** — Luna high · `idiomaticity` · local ID `I1` · canonical issue `SWG-001`.

**Evidence:**

- tests/cli_integration.rs:24-30 names run_saltybox_with_v2_override and explicitly accepts override_value: &str.
- tests/cli_integration.rs:311 and :345 pass invalid values 0 and garbage.
- The controlled environment variable is itself named SALTYBOX_EXPERIMENTAL_V2.

**Assessment:** A shorter generic name is a reasonable preference, but v2 identifies the override variable, not a
guarantee that its value enables v2. The explicit value argument and invalid-value test name make the setup clear. No
false functional contract or repository idiom violation was established.

### F065: Default-engine documentation still claims exclusive write selection

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `SWG-002`.

**Evidence:**

- src/format.rs:84-89 says all newly written files use this engine and calls it the single selection point.
- src/format.rs:107-110 now selects V2Engine directly for Some("1").
- src/bin/saltybox.rs:72-79 passes the selected engine to encrypt and update; the base instead always used
  default_write_engine.

**Assessment:** The comment was accurate for the base write path and becomes false in this change. This is concrete
documentation drift in the write-selection contract, not a request for additional routine docstrings.

### F066: CLI module description mentions only v1 cryptography

**out_of_scope / minor** — Luna high · `docs-comments` · local ID `DOC-002` · canonical issue `SWG-003`.

**Evidence:**

- src/bin/saltybox.rs:1-4 describes encrypting, decrypting, and updating with scrypt and NaCl secretbox.
- The same text is present unchanged in base b2619366c05acabc455d9e677e7dff372790a4e5.
- Base src/format.rs:82 already registers V2Engine for reads; base SPEC.md already specifies v2 decrypt and update
  validation.

**Assessment:** The incomplete description is real, but already misdescribed the CLI's supported decryption and
update-validation cryptography before this change. The proposed expansion to both supported algorithms would already
have been warranted in the base; the exact-diff review excludes this pre-existing defect.

### F067: README omits the newly available experimental write override

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-003` · canonical issue `SWG-004`.

**Evidence:**

- SPEC.md:69-81 adds SALTYBOX_EXPERIMENTAL_V2 selection, invalid-value failure, and the do-not-automate warning.
- README.md:35-57 usage gives no override; :67-68 still describes scrypt/NaCl generally.
- README.md:113 labels the detailed format section as version 1; :125-126 mentions only v2 decryption and SPEC.md.

**Assessment:** The docs charter explicitly covers materially changed user-facing configuration. The newly available
opt-in write command has no usage-level discovery or warning in README. SPEC records the behavior but does not eliminate
that omission. The report somewhat overstates the v1 detail section, which is explicitly scoped to v1; the actionable
new-configuration omission remains valid and minor.

### F068: README omits the newly available experimental write override

**valid / minor** — Terra medium · `docs-comments` · local ID `DOC-2` · canonical issue `SWG-004`.

**Evidence:**

- SPEC.md:69-81 adds SALTYBOX_EXPERIMENTAL_V2 selection, invalid-value failure, and the do-not-automate warning.
- README.md:35-57 usage gives no override; :67-68 still describes scrypt/NaCl generally.
- README.md:113 labels the detailed format section as version 1; :125-126 mentions only v2 decryption and SPEC.md.

**Assessment:** The docs charter explicitly covers materially changed user-facing configuration. The newly available
opt-in write command has no usage-level discovery or warning in README. SPEC records the behavior but does not eliminate
that omission. The report somewhat overstates the v1 detail section, which is explicitly scoped to v1; the actionable
new-configuration omission remains valid and minor.

### F069: New command-wide failure guarantee excludes a real post-replacement error

**valid / material** — Luna high · `docs-comments` · local ID `DOC-004` · canonical issue `SWG-005`.

**Evidence:**

- SPEC.md:18-22 newly applies unchanged-output-on-any-failure to all commands.
- src/file_ops.rs:258-270 persists the temporary file over the target, then :275-282 propagates a directory sync error.
- src/bin/saltybox.rs:82-84 reports the propagated error and exits nonzero.
- Base SPEC.md:18-19 explicitly left encrypt and update write behavior unspecified; its unchanged-on-failure promise
  applied only to decrypt.

**Assessment:** The I/O implementation predates the change, but the new encrypt/update contract does not: broadening the
prior decrypt-only promise introduces an inaccurate data-preservation guarantee. A failed command may already have
replaced the old ciphertext. Correctly documenting the distinct post-rename outcome is actionable without pretending
atomic rename implies rollback.

### F070: New command-wide failure guarantee excludes a real post-replacement error

**valid / material** — Terra medium · `docs-comments` · local ID `DOC-1` · canonical issue `SWG-005`.

**Evidence:**

- SPEC.md:18-22 newly applies unchanged-output-on-any-failure to all commands.
- src/file_ops.rs:258-270 persists the temporary file over the target, then :275-282 propagates a directory sync error.
- src/bin/saltybox.rs:82-84 reports the propagated error and exits nonzero.
- Base SPEC.md:18-19 explicitly left encrypt and update write behavior unspecified; its unchanged-on-failure promise
  applied only to decrypt.

**Assessment:** The I/O implementation predates the change, but the new encrypt/update contract does not: broadening the
prior decrypt-only promise introduces an inaccurate data-preservation guarantee. A failed command may already have
replaced the old ciphertext. Correctly documenting the distinct post-rename outcome is actionable without pretending
atomic rename implies rollback.

## saltybox-v2-flip-initial

[Pinned source](https://github.com/scode/saltybox/tree/ac5de1f97e43881480641d32b32b44f38c1cc5de).

### F071: The test comment overstates removal of the v1 write engine

**valid / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `flip-001`.

**Evidence:**

- src/file_ops.rs:373-377 contains the changed claim that nothing exposes a v1 write engine anymore; Luna quotes this
  accurately but attributes it to src/format.rs.
- src/format.rs:32-37 exposes FormatEngine::encrypt; src/format.rs:60-68 implements v1 encryption; src/format.rs:119-123
  returns that trait object for input beginning saltybox1:.
- src/format.rs:84-92 and 156-160 already qualify their claims to the CLI; those particular anchors are not defective.

**Assessment:** The reported comment contradiction is real at the corrected file_ops anchor: a caller can obtain a v1
encrypt-capable engine through read dispatch. Count the inaccurate test rationale as a minor documentation defect, not a
CLI contract failure. Luna's broader criticism of the already CLI-qualified format.rs comments is unsupported; no
separate fix is needed there.

### F072: Coordinate the README compatibility cutoff with release metadata

**optional / minor** — Luna high · `docs-comments` · local ID `DOC-002` · canonical issue `flip-002`.

**Evidence:**

- README.md:97-98 says versions before 4.0 cannot read saltybox2; Cargo.toml:3 still says 3.3.1 and
  src/bin/saltybox.rs:16-19 derives the CLI version from package metadata.
- CONTRIBUTING.md:33-51 explicitly updates Cargo.toml and Cargo.lock in a separate release PR.
- lore/2026-07-01-saltybox2.md:76-77 explicitly plans this feat! flip to release as 4.0.0.
- The local v3.3.1 tag resolves to b469941eee9c6b368900fb8a70e0b93ac2dea0a9; its src/lib.rs exports only the v1
  secretcrypt/varmor path, and its src/varmor.rs:32-64 rejects saltybox2 as unsupported.

**Assessment:** The development build's displayed version is a real observation, and release coordination is reasonable
advice. The report correctly makes its release-risk premise conditional. But this is a feature commit under an explicit
separate-release workflow, with the major bump already planned; it does not establish a defective release or a required
fix in this diff. Treat changing the wording now as optional clarification.

### F073: Claim that released 3.3.1 already reads saltybox2

**invalid / minor** — Terra medium · `docs-comments` · local ID `DC-1` · canonical issue `flip-002`.

**Evidence:**

- The reviewer identifies the subject's Cargo.toml:3 version as proof of the released implementation, although subject
  ac5de1f97e43881480641d32b32b44f38c1cc5de is a feature commit.
- The actual local v3.3.1 tag is b469941eee9c6b368900fb8a70e0b93ac2dea0a9; its src/file_ops.rs decrypts through
  varmor::unwrap and secretcrypt::decrypt, and src/varmor.rs:32-64 accepts only saltybox1 and rejects saltybox2.
- CONTRIBUTING.md:44-51 assigns version changes to a separate release PR; lore/2026-07-01-saltybox2.md:76-77 plans
  version 4.0.0 for the flip.

**Assessment:** The decisive claim that a released 3.3.1 binary is a counterexample is contradicted by its tagged
source. A development tree retaining the previous package version is not evidence of a release. The suggested
format-support wording is harmless, but the definite compatibility defect and claimed affected released users are not
established.

### F074: Treat the pre-flip development base as a released 3.x version

**invalid / minor** — Terra medium · `correctness-data-flow` · local ID `CDF-1` · canonical issue `flip-002`.

**Evidence:**

- Base 48deffded7561ed58ae6da292e620c202d64e411 does contain V2Engine read support, but the report supplies no release
  evidence for that base.
- The local v3.3.1 tag b469941eee9c6b368900fb8a70e0b93ac2dea0a9 lacks a format_v2 module and rejects saltybox2 through
  src/varmor.rs:32-64.
- CONTRIBUTING.md:33-51 separates feature commits from release version bumps, and lore/2026-07-01-saltybox2.md:76-77
  identifies 4.0.0 as the intended flip release.

**Assessment:** Read support in the preceding development commit does not prove the immediately preceding release
supports v2. The reported recovery consequence depends on that unsupported conversion of commit history into release
history. This is the same proposed README issue as DC-1, preserved separately for lens attribution.

### F075: Nothing exposes a v1 write engine is too broad

**valid / minor** — Muse high · `docs-comments` · local ID `DOC-01` · canonical issue `flip-001`.

**Evidence:**

- src/file_ops.rs:376-377 says the v1 test data uses the frozen modules directly because nothing exposes a v1 write
  engine anymore.
- src/format.rs:119-123 returns V1Engine as a public FormatEngine trait object for saltybox1: input; src/format.rs:37
  and 65-68 retain its callable encryption implementation.
- src/format.rs:89-92 correctly limits the decrypt-only statement to CLI selection.

**Assessment:** The initially possible premise is confirmed by the public trait and dispatcher: v1 encryption is
reachable without directly calling secretcrypt_v1. Qualifying the test rationale to CLI/default selection repairs a
small factual error. This does not require removing v1 encryption or changing user-visible behavior.

## stark-parts-pr56-catalog-static-asset

[Pinned source](https://github.com/scode/stark-parts/tree/2e3e1eaca82736c8d6cf1f5ca4c3f4c919406a0b).

### F076: README still describes compile-time catalog inclusion

**valid / minor** — Luna high · `docs-comments` · local ID `DC-001` · canonical issue `stark56-readme-embedded-catalog`.

**Evidence:**

- README.md:44-45 says the application consumes the catalog with include_str!.
- index.html:8 adds the catalog as a Trunk copy-file asset.
- crates/stark-parts-web/src/main.rs:59-72 fetches and parses the asset at startup; lib.rs:1067-1072 retains
  include_str! only in the test module.

**Assessment:** The change makes the previously accurate README statement false. This is useful architecture and
deployment documentation drift, within the charter's README exception, with minor documentation impact. The review's
index.html:8 anchor is correct.

### F077: README could mention loading and failure screens

**optional / minor** — Luna high · `docs-comments` · local ID `DC-002` · canonical issue
`stark56-readme-startup-states`.

**Evidence:**

- README.md:21-23 describes the successful startup and search flow without promising that loading or failure screens do
  not exist.
- SPEC.md:62-65 explicitly documents the loading and failure behavior.
- crates/stark-parts-web/src/main.rs:35-52 makes these states self-explanatory and tells users to reload on failure.

**Assessment:** The observation is accurate and on scope, but the README is an overview, not an exhaustive screen
inventory. A short addition could help operators, yet the omission does not give incorrect instructions or conceal an
undocumented recovery requirement. Kept separate from the definite stale include_str! statement because it is
independently editable.

### F078: README still describes compile-time catalog inclusion

**valid / minor** — Terra medium · `docs-comments` · local ID `DOCS-1` · canonical issue
`stark56-readme-embedded-catalog`.

**Evidence:**

- README.md:45 explicitly names include_str!.
- index.html:8 copies the catalog as an asset and crates/stark-parts-web/src/main.rs:59-72 fetches it.
- IMPL_SPEC.md:62-66 now requires separate application and catalog artifacts.

**Assessment:** Verified stale documentation caused by this change. It misstates the deployed data path and matches Luna
DC-001 and Muse F1.

### F079: README still describes compile-time catalog inclusion

**valid / minor** — Muse high · `docs-comments` · local ID `F1` · canonical issue `stark56-readme-embedded-catalog`.

**Evidence:**

- README.md:45 names include_str!, while crates/stark-parts-web/src/main.rs:59-72 fetches the deployed catalog.
- The Trunk copy-file declaration is actually index.html:8, not index.html:10 as this report states.
- IMPL_SPEC.md:62-66 explains why code and catalog artifacts are now separate.

**Assessment:** The central finding is correct despite a small source-line error. Its suggested extra README explanation
of loading/failure is ancillary to fixing the stale sentence, not an independently reported finding; the raw report
remains intact.

### F080: Startup and permanent failure screens omit the required unofficial-site notice

**valid / material** — Muse high · `correctness-data-flow` · local ID `F1` · canonical issue `stark56-startup-banner`.

**Evidence:**

- SPEC.md:18-20 requires a persistent, visually distinct unofficial-site banner visible on initial page load.
- crates/stark-parts-web/src/main.rs:21-25 mounts App only on successful catalog loading.
- crates/stark-parts-web/src/main.rs:35-55 renders loading and failure screens without the notice.
- crates/stark-parts-web/src/lib.rs:75-79 contains the notice only inside AppWithInitialState.
- The base main.rs mounted App directly; the changed startup flow creates new reachable states without that banner.

**Assessment:** The reviewer labeled the spec reading possible. The explicit persistent and initial-load requirements
establish its premise: they contain no startup exception, and the new loading/failure clause does not remove them. A
failed asset request leaves the site titled Stark Parts indefinitely without its required attribution and authority
disclosure. This is a consequential user-visible contract regression, not a request to restore search controls before
data is ready.

### F081: Startup views omit the persistent unofficial-site notice

**valid / material** — Terra medium · `correctness-edge-inputs` · local ID `CEI-1/banner` · canonical issue
`stark56-startup-banner`.

**Evidence:**

- Original raw finding CEI-1 combines independently editable banner and search-control concerns; this record assesses
  its banner portion.
- SPEC.md:18-20 requires a persistent banner on initial load.
- crates/stark-parts-web/src/main.rs:35-55 has no notice in loading or failure views, while lib.rs:75-79 renders it only
  after App mounts.

**Assessment:** The persistent notice regression is verified and matches Muse data-flow F1. Splitting the original
report preserves its valid disclosure finding without accepting its separate demand for pre-load search controls.
Original text remains in the raw artifact.

### F082: Search controls must be visible before the catalog loads

**uncertain / minor** — Terra medium · `correctness-edge-inputs` · local ID `CEI-1/controls` · canonical issue
`stark56-preload-search-controls`.

**Evidence:**

- Original raw finding CEI-1 also asks for the search shell before loading completes, or a SPEC change.
- The same change already adds SPEC.md:62-65, explicitly defining loading and failure states before search
  initialization.
- crates/stark-parts-web/src/main.rs:32 documents withholding interactive search controls until ready.
- tests/static-smoke.spec.mjs:129 explicitly expects no Search control after load failure; lib.rs:58-60 still focuses
  the search input when it mounts.

**Assessment:** The report offers both a startup search shell and clarification of conflicting Core Experience
requirements. SPEC.md:22-27 still requires an initial search experience and page-load focus, while the new SPEC.md:62-65
expressly adds pre-ready loading and failure states without explicitly resolving those older words. The implementation
and test show the intended control gating but cannot override the normative spec. Whether page load here means initial
rendering or completion of initialization remains ambiguous; no successful-load focus failure is demonstrated. Retain
uncertain/minor for this distinct contract-clarification claim. Parent independently checked this interpretation before
the assessment was finalized.

### F083: Startup states omit the required unofficial-site notice

**valid / material** — Sol high · `correctness-edge-inputs` · local ID `CEI-1` · canonical issue
`stark56-startup-banner`.

**Evidence:**

- SPEC.md:18-20 requires the disclaimer to be persistent and visible on initial page load.
- crates/stark-parts-web/src/main.rs:21-25 gates App on a successful catalog result.
- crates/stark-parts-web/src/main.rs:35-55 has title and loading/error text without the notice.
- crates/stark-parts-web/src/lib.rs:75-79 contains the notice only in the successfully loaded search app.

**Assessment:** Verified and matched to Muse data-flow F1 and the banner portion of Terra edge CEI-1. Sol correctly
separates the always-visible disclaimer from catalog-dependent search controls. Its safety-critical wording is stronger
than needed; the material classification rests on a persistent user-visible attribution/authority contract violation.

## stark-parts-pr57-catalog-only-ci

[Pinned source](https://github.com/scode/stark-parts/tree/81d37ce6d263b3c62815787170b04e304490d249).

### F084: Clarify that catalog-only formatting uses the whole dprint configuration

**optional / minor** — Luna high · `docs-comments` · local ID `DOC-001` · canonical issue `SP57-002`.

**Evidence:**

- IMPL_SPEC.md:76-79 says catalog-only changes validate the catalog crate and generated-file formatting, without
  compiling/testing the web application; it does not say no other formatting is checked.
- .github/workflows/ci.yml:133-137 runs the catalog crate tests and dprint/check with config-path dprint.json, without
  file arguments.
- dprint.json:1-23 configures JSON, Markdown, and TOML plugins with broad exclusions and no catalog-only includes.
- evidence/dprint-check-v2.3-action.yml:29-32 executes dprint check --config with the supplied path and any args.

**Assessment:** The broader dprint scope is real, so explicitly naming repository formatting could improve precision.
However, checking all configured files includes generated files and does not violate the stated no-web-compilation
contract. The paragraph never promises exclusive formatting coverage. No concrete failing unrelated file is established.
Treat this as optional clarification, not a definite documentation defect.

### F085: The every quantifier allegedly skips mixed changes

**invalid / minor** — Muse high · `docs-comments` · local ID `DC-1` · canonical issue `SP57-003`.

**Evidence:**

- .github/workflows/ci.yml:24-31 uses every across the app filter's positive pattern and two negations.
- evidence/paths-filter-v3-filter.ts:91-105 filters each individual file and evaluates patterns.every(aPredicate) for
  that file.
- evidence/paths-filter-v3-main.ts:212-227 exports each filter as files.length > 0.
- A mixed catalog plus source change has at least one source file satisfying all app patterns, so app=true and full jobs
  run under .github/workflows/ci.yml:33-115.

**Assessment:** The finding confuses AND across patterns for one path with AND across all changed paths. The action
exports true if any changed file passes all app patterns; mixed changes therefore run the full suite exactly as
documented. Its proposed switch to the default some would make the leading ** pattern match catalog files too and defeat
the optimization. The attached request to comment the quantifier does not independently establish a missing explanatory
contract.

### F086: Catalog-only CI skips tests that depend on the changed catalog

**valid / material** — Muse high · `correctness-edge-inputs` · local ID `F1` · canonical issue `SP57-001`.

**Evidence:**

- .github/workflows/ci.yml:64-80 and 91-115 now skip cargo test and browser tests for catalog-only changes; :117-137
  runs only the catalog crate tests and formatting.
- crates/stark-parts-web/src/search.rs:947-980 parses the committed catalog and requires Frame cable holder with SKU
  SMX1-WH-F-01 and the first wiring-harness result name Wiring harness EX.
- tests/static-smoke.spec.mjs:63-69 requires Stark VARG toolbox for SMX1-TOOLBOX; :114-116 requires Frame cable holder.
- crates/stark-parts-catalog/src/lib.rs:1260-1374 validates metadata, relations, and safe URLs; :1863-1940 checks exact
  bike IDs, nonempty contents, specific URL shapes, and canonical JSON5, but no product display-name expectation.
- A canonical catalog-only rename of Frame cable holder or Stark VARG toolbox leaves catalog structural/format checks
  satisfied but breaks those existing web/browser assertions.

**Assessment:** The new job gates introduce a concrete validation regression: changed input to existing tests is no
longer checked, allowing a catalog PR to leave the normal suite failing until a later app change runs it. This is not a
demand to undo the intended compile-saving policy: move compatibility assertions into a lightweight validation path or
decouple web tests from mutable catalog data. The raw report's bike-count example is wrong because catalog tests enforce
exactly four known variants, and catalog-check runs more than a single parse test; the independent name/SKU examples
establish the finding. No current production search failure is claimed, and no tests were executed.

### F087: Shallow checkout allegedly makes push classification unreliable

**invalid / minor** — Muse high · `correctness-edge-inputs` · local ID `F2` · canonical issue `SP57-004`.

**Evidence:**

- .github/workflows/ci.yml:20 uses the default shallow actions/checkout.
- evidence/paths-filter-v3-main.ts:104-155 resolves the pushed default-branch comparison to before SHA versus head.
- evidence/paths-filter-v3-git.ts:18-33 calls ensureRefAvailable for both endpoints and performs a two-dot diff.
- evidence/paths-filter-v3-git.ts:216-235 fetches missing endpoints with depth=1 and throws if unavailable;
  main.ts:49-51 marks an exception as a failed action.

**Assessment:** The action intentionally handles missing history by fetching the exact comparison endpoint. No concrete
failure in that supported path is shown, and this push path does not fall back to treating all files as changed. A
generic network-fetch failure is not evidence that full history is required; checkout itself also fetches. The suggested
fetch-depth:0 is unsupported hardening, not an actionable introduced bug.

### F088: Catalog-only CI skips committed-catalog web integration assertions

**valid / material** — Sol high · `correctness-data-flow` · local ID `CDF-1` · canonical issue `SP57-001`.

**Evidence:**

- .github/workflows/ci.yml:64-80 gates the complete cargo test job on app changes; :117-137 restricts catalog-only
  execution to stark-parts-catalog.
- crates/stark-parts-web/src/search.rs:927-944 loads catalog/stark-parts.json5 and asserts an SMX1-TOOLBOX search hit;
  :947-980 asserts specific names and ranking against committed data.
- crates/stark-parts-catalog/src/lib.rs:1863-1940 checks exact bike variants, metadata, nonempty content, URLs and
  canonical JSON5, but does not enforce the web tests' SKU or display-name expectations.
- The base .github/workflows/ci.yml:39-54 ran cargo test unconditionally.

**Assessment:** The input to skipped tests remains mutable in catalog-only PRs. A canonical rename of a required part
display name is a concrete catalog-valid input that leaves the ordinary suite failing while the new reduced job passes.
Preserving a lightweight compatibility check or isolating volatile data from web-test expectations can satisfy the
no-web-compilation policy. This matches Muse edge F1 at the same newly removed coverage boundary, without its
unsupported bike-count example.

## stark-parts-pr58-catalog-vercel-cache

[Pinned source](https://github.com/scode/stark-parts/tree/4937a788724b8c85f82f03ed93ac59f27ef51945).

### F089: Qualify the README toolchain-install description for cache hits

**optional / minor** — Terra medium · `docs-comments` · local ID `DOCS-1` · canonical issue
`stark58-readme-cache-hit-description`.

**Evidence:**

- README.md:59-65 still instructs operators to import the root project using the Other framework and checked-in
  vercel.json, then describes Rust and Trunk installation as part of the build.
- package.json:5-7 runs turbo run build:app followed by the uncached assemble:catalog script; scripts/build-app.sh:5-21
  places Rust and Trunk setup inside build:app.
- IMPL_SPEC.md:73-78 explicitly documents cache restoration, uncached catalog assembly, and avoiding toolchain
  installation on a cache hit.
- vercel.json:4-7 preserves the configured build entrypoint and dist output while changing installation to npm ci;
  README.md:74-78 already lists npm ci and npm run build.
- charters/docs-comments.md, README drift section, limits findings to changed user-visible behavior, flags,
  configuration, or setup steps and excludes documenting implementation details for internal refactors.

**Assessment:** The factual observation is sound: a cache hit skips the script, so the README paragraph could describe
the new path more precisely. It does not give an incorrect operator command, require an obsolete setup action, or
conceal a changed deployment requirement. The build still owns installation when compilation is needed, and the cache
detail is explicitly documented in IMPL_SPEC.md. Under this charter the suggested README expansion is an optional
precision improvement, not an actionable documentation defect. This is not rejected for low severity; it is optional
because the required user-facing setup contract remains intact.
