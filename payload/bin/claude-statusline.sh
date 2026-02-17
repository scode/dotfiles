#!/bin/sh
#
# Claude Code status line script.
# Called by Claude Code with JSON piped to stdin.

set -eu

input=$(cat)
cwd=$(echo "$input" | jq -r '.workspace.current_dir')
current_dir=$(basename "$cwd")
model=$(echo "$input" | jq -r '.model.display_name')

# --- Git info ---

git_info=""
if git -C "$cwd" rev-parse --git-dir > /dev/null 2>&1; then
  branch=$(git -C "$cwd" branch --show-current 2>/dev/null) \
    || branch=$(git -C "$cwd" rev-parse --short HEAD 2>/dev/null) \
    || branch=""

  if [ -n "$branch" ]; then
    staged=$(git -C "$cwd" diff --cached --numstat 2>/dev/null | wc -l | tr -d ' ')
    modified=$(git -C "$cwd" diff --numstat 2>/dev/null | wc -l | tr -d ' ')

    counts=""
    [ "$staged" -gt 0 ] && counts="+$staged"
    [ "$modified" -gt 0 ] && counts="$counts ~$modified"

    git_info=" git:($branch)$counts"
  fi
fi

# --- Context window bar ---

context_info=""
used=$(echo "$input" | jq -r '.context_window.used_percentage // 0')

if [ -n "$used" ]; then
  used_int=$(printf "%.0f" "$used")
  filled=$((used_int / 10))
  empty=$((10 - filled))

  bar=""
  i=0; while [ $i -lt $filled ]; do bar="${bar}▓"; i=$((i + 1)); done
  i=0; while [ $i -lt $empty ];  do bar="${bar}░"; i=$((i + 1)); done

  if [ "$used_int" -lt 70 ]; then
    color="\033[32m"   # green
  elif [ "$used_int" -lt 90 ]; then
    color="\033[33m"   # yellow
  else
    color="\033[31m"   # red
  fi

  context_info=$(printf " %b[%s]\033[0m" "$color" "$bar")
fi

# --- Output ---
# Model and context bar come before git info so they stay in a roughly
# consistent horizontal position regardless of branch name length.

printf "\033[1;32m➜\033[0m  \033[36m%s\033[0m %s%s%s" \
  "$current_dir" "$model" "$context_info" "$git_info"
