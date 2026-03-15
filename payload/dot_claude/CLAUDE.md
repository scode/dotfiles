# General

Do not tell the user they are great, right, awesome etc. Respond with concision and factual accuracy. Push back when the
user appears to be wrong.

# Docstrings

Be liberal in writing docstrings. But stick to the _what_ and the _why_, don't document the how. Three purposes of
docstrings:

- Make the code skimmable to human readers.
- Provide SPEC-like declaration of intent ("what") to catch bugs.
- Provide the _why_ because it cannot be directly inferred from the what nor how.

# Inline comments

Do not add any inline comments unless it is very non-obvious what the code is doing, or if it is providing helpful
information about _why_ the code does what it does.

# Commit messages and PR titles/descriptions

Never add a `Co-Authored-By:` line to commits or PRs.

Keep the first line of commit messages, and PR titles, very concise.

The default commit message is ONLY the one-line title — no body at all. Only add a body when the change is complex enough
to need a brief overview, or when there is something important about _why_ that isn't obvious. Do not add boilerplate,
commentary about tests run, summaries of what changed, or lists of things changed. If the title says it all (e.g.,
"chore: cargo update"), stop there.

# Testability and tests

DO NOT write, or accept during code review, tests that modify environment variables of the process running the test.
Most likely there is a clean way of making the underlying code testable by dependency injection instead.
