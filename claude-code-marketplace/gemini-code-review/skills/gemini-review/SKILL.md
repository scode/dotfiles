---
name: gemini-review
description: Have Google's Gemini CLI review the current changes
---

# Gemini Code Review

Dispatch the `gemini-code-review:gemini-code-review` agent to review current changes using Google's Gemini CLI.

## Instructions

Use the Task tool to dispatch the `gemini-code-review:gemini-code-review` subagent with the following prompt:

```
Review the current changes. {ADDITIONAL_CONTEXT}
```

Where `{ADDITIONAL_CONTEXT}` is any context the user provided when invoking the skill (e.g.,
`/gemini-review focus on error handling`).

If no additional context was provided, simply ask the agent to review the current changes.
