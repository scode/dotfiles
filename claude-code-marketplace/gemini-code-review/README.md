# gemini-code-review

A Claude Code plugin that invokes Google's Gemini CLI to review code changes.

## Prerequisites

- The `gemini` CLI must be installed and configured with valid API credentials
- `jq` must be available for the session-start hook
- Claude Code with plugin support

## How It Works

### The Problem

Claude Code plugins can define agents that run bash commands, but those commands don't have access to
`CLAUDE_PLUGIN_ROOT` - the environment variable that points to the plugin's installation directory. This is a problem
because plugins are installed to versioned cache directories (e.g.,
`~/.claude/plugins/cache/dotfiles/gemini-code-review/45dd31bf8d5d/`) and we need a way to locate the `bin/review`
script.

### The Solution

We use Claude Code's hook system to inject the plugin path into the session context:

1. **SessionStart hook** (`hooks/hooks.json`): When a Claude Code session starts, this hook runs `session-start.sh`.

2. **Path injection** (`hooks/session-start.sh`): The script determines the plugin root directory (using
   `CLAUDE_PLUGIN_ROOT` if available, otherwise deriving it from the script's own location) and outputs JSON that
   injects the bin path into the session's additional context:
   ```
   GEMINI_REVIEW_BIN=/full/path/to/plugin/bin
   ```

3. **Agent usage** (`agents/code-review.md`): The agent reads `GEMINI_REVIEW_BIN` from its session context and uses the
   literal path in bash commands. This means the user sees the full path and can approve it safely.

### Why This Matters for Security

The user sees exactly which script will be executed (the full path), rather than an opaque
`$CLAUDE_PLUGIN_ROOT/bin/review`. This allows informed approval of bash commands.

## Plugin Structure

```
gemini-code-review/
├── .claude-plugin/
│   └── plugin.json       # Plugin metadata
├── agents/
│   └── code-review.md    # Agent definition
├── bin/
│   └── review            # The actual review script
├── hooks/
│   ├── hooks.json        # Hook definitions
│   └── session-start.sh  # Injects GEMINI_REVIEW_BIN into context
└── README.md
```

## The Review Script

`bin/review` does the following:

1. Gets the current diff (`git diff HEAD`, falling back to branch diff)
2. Writes it to a temp file
3. Invokes `gemini -s -p` with a review prompt and restricted tools (read-only)
4. Cleans up the temp file

You can pass additional context as arguments:

```bash
/path/to/bin/review "Focus on error handling" "This is a new auth module"
```

## Installation

Add to your Claude Code settings:

```json
{
  "enabledPlugins": {
    "gemini-code-review@your-marketplace": true
  }
}
```

## Usage

Ask Claude to have Gemini review your code:

- "Have Gemini review this code"
- "What does Gemini think of these changes?"
- "Get Gemini's opinion on my implementation"
