#!/usr/bin/env bash
set -euo pipefail

file="${ZED_FILE}"
row="${ZED_ROW}"
col="${ZED_COLUMN}"
sel="${ZED_SELECTED_TEXT:-}"

# Zed uses zero-indexed "row" internally (common in its APIs).  [oai_citation:6‡Zed](https://zed.dev/blog/zed-decoded-text-coordinate-systems?utm_source=chatgpt.com)
# We'll print 1-based line numbers for human readability.
one_based_line=$((row + 1))

if [[ -n "${sel}" ]]; then
  payload=$(cat <<EOF
[Zed → Claude context]
File: ${file}
Cursor: line ${one_based_line}, col ${col}

Selected text:
\`\`\`
${sel}
\`\`\`
EOF
)
else
  # Show a window around the cursor
  start=$((one_based_line - 80)); [[ $start -lt 1 ]] && start=1
  end=$((one_based_line + 80))

  # Print with line numbers
  excerpt=$(nl -ba "${file}" | sed -n "${start},${end}p")

  payload=$(cat <<EOF
[Zed → Claude context]
File: ${file}
Cursor: line ${one_based_line}, col ${col}
Context: lines ${start}-${end}

\`\`\`
${excerpt}
\`\`\`
EOF
)
fi

# macOS clipboard
printf "%s" "${payload}" | pbcopy
echo "Copied Claude context to clipboard."
