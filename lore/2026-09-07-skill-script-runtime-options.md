# Runtime options for deterministic skill helpers

NOTE: Historical discussion recorded 2026-09-07, not an implementation plan or a runtime selection. This captures the
options considered for speeding up the brain skillette and making its storage operations more reliable. Not maintained.

## Why this came up

The brain uses GitHub as mechanical storage for Markdown artifacts and a concise `BRAIN.md` index. Its instructions
make the agent manage a centralized local cache, isolated worktrees, session snapshots, commits, push retries, and
conflicts. A write is not stored until it has reached the remote; a local commit alone is not success.

The proposed improvement was to put the repeatable Git/GitHub mechanics into tested code, leaving the agent to find and
edit content and resolve semantic conflicts. This should reduce tool round trips and opportunities for shell mistakes,
especially with cheaper models. No latency improvement was measured. The existing cache-lock shell helper was a small
step in this direction, not a complete storage implementation.

Peter wanted something lightweight with expressive static types. The objection to Python was not that it is technically
weakly typed; it was that its typing, even with annotations, was not powerful enough for his preference. Rust had the
desired guarantees, but a separate Rust project's build and distribution overhead felt high for skill helpers.

## Language and runtime options

These are the tradeoffs discussed, not a comprehensive language comparison.

| Option | What makes it attractive | What works against it here |
| --- | --- | --- |
| Shell | Already available in the local execution environment; direct access to Git and other commands; little setup. | No useful static model of operation states. Quoting, exit handling, cleanup, and concurrency become hard to audit as the program grows. Suitable for small wrappers, not the preferred home for the whole storage protocol. |
| Python with a strict type checker | Low ceremony, broad library support, straightforward subprocess and filesystem code. Annotations and a checker improve on untyped scripts. | Does not meet Peter's preference for a more expressive static model. The checker is another step, and interpreter/dependency availability still needs handling. |
| Go | Static typing, a substantial standard library, and a native executable. A plausible compromise between scripting convenience and Rust's project overhead. | Less expressive modeling of variant states and exhaustive handling than Rust; error handling relies heavily on convention. Either install the toolchain or distribute binaries for the target platforms. |
| Rust | Enums, exhaustive matching, `Result`, and ownership make lifecycle and failure contracts explicit. Native binaries need no language runtime on the target. | Cargo setup, builds, dependencies, and platform releases are practical overhead. Using the existing Rust workspace could amortize setup, but would couple the helper's build and distribution to this repository. |
| TypeScript with Node.js | Expressive unions and narrowing, a large ecosystem, and a runtime commonly present in development environments. | Runtime TypeScript support is not type checking. The checker and project conventions still need arranging; Node availability is not universal. |
| TypeScript with Deno | TypeScript execution plus checking, testing, formatting, and linting in one installed tool. Can start with a small script beside the skill and grow into modules without introducing a separate build pipeline. | Deno must be installed or bundled. TypeScript does not provide Rust's guarantees, and runtime permissions need careful handling when the helper launches Git. |

The working conclusion was not that one of these is universally available. It was that provisioning one runtime through
the dotfiles installer might be cheaper than trying to avoid all runtime prerequisites.

## What Deno buys, and what it does not

Deno is a JavaScript/TypeScript runtime with integrated development tools. The attractive part here was TypeScript's
ability to represent distinct outcomes without a pile of unrelated optional fields. For example, a proposed publish
result could distinguish `published` with a commit ID, `conflict` with a worktree and conflicting paths, and `failed`
with recovery information. Narrowing on the result kind makes the appropriate fields available; exhaustive handling
can be enforced with the usual `never` pattern. This was an interface sketch, not an API commitment.

`deno check`, `deno test`, `deno fmt`, and `deno lint` cover the basic development loop. Plain `deno run` does not check
types by default; `deno run --check` or a separate check is needed. Deno's type checker uses strict mode by default.
These details were checked against the [Deno TypeScript documentation](https://docs.deno.com/runtime/fundamentals/typescript/).

TypeScript types are erased. Assertions and `any` can bypass checking, and external JSON still needs runtime validation.
It has neither Rust's ownership model nor mandatory typed exception handling. Expressive types help describe the state
machine; they do not prove that a Git command did what the helper believes it did. See the
[TypeScript narrowing documentation](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).

Deno denies sensitive capabilities unless granted, but allowing it to launch Git does not sandbox Git under the same
filesystem or network restrictions. Subprocess permissions are therefore a meaningful boundary, not a way to make an
arbitrary Git invocation harmless. See [Deno security](https://docs.deno.com/runtime/fundamentals/security/).

Distribution could mean installing Deno once or using `deno compile` to produce a standalone executable. The latter
bundles the runtime and brings back platform-specific binary distribution; it is not a tiny native program in the Go
or Rust sense. See [Deno compile](https://docs.deno.com/runtime/reference/cli/compile/).

## Cross-harness standardization

The initial framing that there was little standardization was too pessimistic. The shared
[Agent Skills specification](https://agentskills.io/specification) already supports bundled executable code in
`scripts/`, and a skill directory can contain other files and directories. Substantial, multi-module helpers are not
excluded by the format.

The official [script authoring guide](https://agentskills.io/skill-creation/using-scripts) goes beyond a directory
convention. It describes self-contained scripts with dependencies, including Python with `uv` inline metadata and Deno
with versioned `npm:` and `jsr:` imports. It also recommends relative script references, noninteractive interfaces,
`--help`, useful errors, structured stdout, and diagnostics on stderr. This supports the proposed bundled-helper shape.

What the shared specification did not provide was a universal execution contract that provisions a runtime and its
dependencies and exposes typed operations identically across harnesses. `compatibility` is descriptive text, not an
installation manifest. Language support depends on the host, and `allowed-tools` is experimental with implementation
differences. Portable packaging is further along than portable execution.

OpenAI's [skill documentation](https://learn.chatgpt.com/docs/build-skills) also described tool dependencies in
`agents/openai.yaml` and plugin distribution alongside MCP connections. Those are vendor-specific facilities, not
evidence that every harness provisions a bundled program's execution environment the same way.

## Where the discussion stopped

The assistant's recommendation was a bundled TypeScript helper with Deno provisioned once through dotfiles, a small
command interface, and structured results. Peter asked to preserve the options for later; he did not select Deno or
authorize the implementation in this discussion.

Any implementation would still need to preserve the brain's existing contracts: explicit access only; reuse the session
snapshot unless refresh is requested, warranted by evidence, or more than six hours stale; isolate concurrent local
operations; never force-push over another writer; and verify publication before reporting a write stored. Deterministic
code should own the mechanics and bounded retries, but must return semantic conflicts to the agent rather than guess at
which content to discard. Pending work must remain recoverable after failure.

The next useful experiment would be one bounded helper operation with tests, followed by user-requested cheap-model
evals against the existing brain eval expectations. The open choice is whether Deno's lower setup cost is worth the
runtime prerequisite and weaker guarantees than Rust, or whether an existing-workspace Rust implementation makes those
costs small enough to prefer it. Go remains the simpler native-binary compromise.
