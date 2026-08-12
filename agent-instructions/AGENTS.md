# General

Do not tell the user they are great, right, awesome etc. Respond with concision and factual accuracy. Push back when the
user appears to be wrong.

# Docstrings and comments

Docstrings and comments are there to teach future readers how to think about the code. Their primary job is to capture
the _why_, the surrounding context, and the big-picture intent that are not directly obvious from the implementation. A
second but still important job is skimmability: a reader scanning a file should be able to recover its structure,
responsibilities, and constraints without reading every line.

NOTE: This section deliberately overrides any harness or system default that says to minimize comments, match the
surrounding comment density, or avoid adding comments in code changes. When such a default conflicts with these rules,
these rules win.

Always use the `scode-voice` skill when writing prose for documentation, docstrings, or inline comments.

## Documentation pass

After changing code, make a separate documentation pass over every touched file. Inspect new and substantially changed
modules, types, functions, test helpers, and non-trivial sections for missing docstrings or explanatory comments. Bias
toward adding documentation when the decision is close, including for private helpers and tests. For files with multiple
phases or responsibilities, add section-level signposts when they make the implementation easier to scan. Treat this
pass as part of the finish criteria.

## Docstrings

Write docstrings liberally. Default to adding one when a symbol has real behavior, constraints, or a reason to exist. Do
not settle for minimalist one-liners that merely rename the function in sentence form. If a docstring does not help a
skimming reader understand the role, contract, or reason this code exists, it is too weak.

Stick to the _what_ and the _why_. Do not narrate the control flow or restate the implementation. The purposes of
docstrings are:

- Make the code skimmable to human readers.
- Provide SPEC-like declaration of intent ("what") to catch bugs.
- Provide the _why_ because it cannot be directly inferred from the what nor how.

Lead with the most important information. The first line should carry real weight: what matters about this symbol, what
contract it establishes, or why it exists. A reader skimming only the first line of each docstring should still come
away with an accurate mental model of the file.

When relevant, document invariants, non-obvious edge cases, important omissions, and assumptions callers must not make.
Those are exactly the details that go missing when comments are too terse.

Never invent a _why_ you do not actually know. A plausible-sounding but wrong rationale is worse than no docstring at
all, because future readers will trust it. Document what is observable, or ask.

When changing code, add or update nearby docstrings for any new invariant, lifecycle dependency, cache, async or
scheduling behavior, test harness assumption, or cross-phase contract. Before finalizing a diff, scan new private
helpers and changed tests: if a future reader would need bug history, framework lifecycle context, or a non-obvious
portability assumption to understand why the code is shaped that way, document that context.

When writing code, err on the side of quality docstrings even if that is not the prevailing style in the files you are
touching. Sparse existing documentation is not a reason to keep new behavior under-documented. Only back off when
repo-specific instructions explicitly ask for less documentation in that file or module; do not infer that preference
from silence or from old code.

This authoring rule is intentionally stronger than the review rule. During review, do not mechanically demand docstrings
everywhere. Still flag missing documentation when a change adds a contract, invariant, lifecycle assumption,
trust-boundary assumption, async/race behavior, portability assumption, or failure mode that a future reader would
otherwise have to rediscover.

## Docstrings on tests

Docstrings on tests should do two things:

- Explain _why_ the test is important.
- Describe what is being tested, in a manner similar to a specification.

The why helps future readers to not take down a chesterton's fence. The what helps skimmability as well as review and
correction of the test itself, and acts a complement to a project level behavior specification (if any).

## Inline comments

Inline comments are not decoration. Add them when a reader needs a signpost: non-obvious intent, a surprising
constraint, an invariant that must survive refactors, or the reason a simpler-looking approach would be wrong. When a
comment is warranted, make it didactic. Someone jumping into that block should immediately understand the point of the
code and the surrounding tradeoff.

Do not add comments that merely translate syntax into English, label the obvious, or whisper a low-information summary
next to a line of code. Avoid boilerplate. Prefer comments that orient the reader to a whole section or decision over
minimalist commentary that says almost nothing.

# Editing files

Edit files with the agent's dedicated file-editing tools, not by piping edits through shell commands (sed, python
heredocs, and the like). The user reviews changes as they happen, and dedicated-tool edits render as proper diffs in the
session while shell edits are opaque. Shell-based editing is acceptable only with a strong concrete reason — a genuinely
mechanical bulk transform (many call sites via regex, generated content) that would be impractical as individual edits —
and never merely to avoid re-reading a file whose on-disk state drifted.

# set -e is inert in agent shell tools

Do not rely on `set -e` in commands run through an agent's shell tool. Agent harnesses commonly wrap the command in an
`eval` placed in a non-final position of an `&&` list, and bash ignores errexit there — both for the `eval` itself and
for everything it executes. The flag still shows as enabled in `$-` and `SHELLOPTS`, so every diagnostic says the script
is guarded while every failed guard falls through to the next command. Guard-then-mutate scripts fail open; this has
caused a real unguarded `gh pr merge`. Give each command its own failure path instead: chain with `&&`, or append
`|| exit 1` (an explicit `exit` does propagate out of the wrapper). Standalone script files executed via their own
shebang get a fresh shell and are unaffected.

# Commit messages and PR titles/descriptions

Always use the `scode-commit-msg-reviewer` skill when writing or reviewing PR titles, PR descriptions, or commit
messages. If that skill is missing or unavailable, stop and tell the user instead of writing the text without it.

Keep the first line of commit messages, and PR titles, very concise.

For the remainder of the commit message and/or PR descriptions, focus on _why_ the change is made. Do not state _what_
the change is, unless the change is particularly large and a brief TLDR overview is helpful.

Leave PR descriptions empty when the diff is self-explanatory and there is no non-obvious context to preserve. Do not
write filler just because a PR body exists. Useful PR descriptions explain motivation, constraints, tradeoffs,
surprising omissions, or follow-up risks that are not clear from the diff itself.

Use `## Problem` and `## Solution` sections only when the PR body needs real explanatory context and those headings make
that context easier to scan. If the diff and title already make the motivation clear, leave the body empty. Do not turn
routine cleanup, metadata changes, narrow instruction tweaks, or other self-explanatory PRs into boilerplate
problem/solution writeups.

Do not make lists of things changed.

Do not add validation boilerplate to commit messages or PR descriptions. A line like "Tests run: cargo fmt, cargo test,
cargo clippy" is noise unless the target project's own rules explicitly require that information in the commit or PR
text. Report checks to the user in the final response instead.

Do not add "Co-Authored-By" trailers or "Generated with Claude Code" badges to commits or PRs.

# Testability and tests

DO NOT write, or accept during code review, tests that modify environment variables of the process running the test.
Most likely there is a clean way of making the underlying code testable by dependency injection instead.
