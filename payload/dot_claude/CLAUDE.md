# General

Do not tell the user they are great, right, awesome etc. Respond with concision and factual accuracy. Push back when the
user appears to be wrong.

# Docstrings and comments

Docstrings and comments are there to teach future readers how to think about the code. Their primary job is to capture
the _why_, the surrounding context, and the big-picture intent that are not directly obvious from the implementation. A
second but still important job is skimmability: a reader scanning a file should be able to recover its structure,
responsibilities, and constraints without reading every line.

Always use the `scode-voice` skill when writing prose for documentation, docstrings, or inline comments.

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

## Inline comments

Inline comments are not decoration. Add them when a reader needs a signpost: non-obvious intent, a surprising
constraint, an invariant that must survive refactors, or the reason a simpler-looking approach would be wrong. When a
comment is warranted, make it didactic. Someone jumping into that block should immediately understand the point of the
code and the surrounding tradeoff.

Do not add comments that merely translate syntax into English, label the obvious, or whisper a low-information summary
next to a line of code. Avoid boilerplate. Prefer comments that orient the reader to a whole section or decision over
minimalist commentary that says almost nothing.

# Commit messages and PR titles/descriptions

Keep the first line of commit messages, and PR titles, very concise.

For the remainder of the commit message and/or PR descriptions, focus on _why_ the change is made. Do not state _what_
the change is, unless the change is particularly large and a brief TLDR overview is helpful.

Leave PR descriptions empty when the diff is self-explanatory and there is no non-obvious context to preserve. Do not
write filler just because a PR body exists. Useful PR descriptions explain motivation, constraints, tradeoffs,
surprising omissions, or follow-up risks that are not clear from the diff itself.

Do not make lists of things changed.

Do not add validation boilerplate to commit messages or PR descriptions. A line like "Tests run: cargo fmt, cargo test,
cargo clippy" is noise unless the target project's own rules explicitly require that information in the commit or PR
text. Report checks to the user in the final response instead.

Do not add "Co-Authored-By" trailers or "Generated with Claude Code" badges to commits or PRs.

# Testability and tests

DO NOT write, or accept during code review, tests that modify environment variables of the process running the test.
Most likely there is a clean way of making the underlying code testable by dependency injection instead.
