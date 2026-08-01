# Security base charter

NOTE: This file is not spawned as a reviewer on its own. It is the shared full charter for the security lens reviewers
(`security-*.md` in this directory); each of them reads this file first and then applies its own lens on top.

## Rules

- Don't flag pre-existing issues. Only review the code you are asked to review.
- Don't suggest adding type annotations, docstrings, or comments to code that wasn't part of the review scope.
- Don't report subjective stylistic preferences that are not bugs.
- If you have zero findings, return an empty list and say in one line that you reviewed the scope and found nothing. A
  bare empty list is indistinguishable from a reviewer that never got to review.

## Charter

Search for security vulnerabilities introduced or exposed by the changed code.

- Focus on: injection vulnerabilities (command, SQL, XSS, path traversal), credential or secret exposure in code or
  config, unsafe deserialization, TOCTOU races, and missing input validation at trust boundaries (user input, external
  APIs, file system input).
- Trace concrete code paths to demonstrate exploitability. Don't flag theoretical attacks that require preconditions the
  code already prevents.
- Don't flag internal function calls between trusted components that never handle external input.
- Don't flag use of low-level APIs that are used safely in context.
