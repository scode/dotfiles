#!/usr/bin/env python3

import argparse
import shlex
import sys
from pathlib import Path

ALLOWED_EXISTING_ENTRIES = {".git", ".hg", ".sl"}

AGENTS_CONTENT = """# TODO Workspace Agent Guide

- Always use the `$scode-todo` skill in this repository.
"""

CLAUDE_CONTENT = """# TODO Workspace Agent Guide

- Always use the `$scode-todo` skill in this repository.
"""

TODO_CONTENT = """# TODO

## Timed TODOs

## Untimed TODOs
"""

DONE_CONTENT = """# DONE

## Completed TODOs
"""

T_MAX_CONTENT = "0\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Initialize an empty repository as a scode-todo project."
    )
    parser.add_argument(
        "--path",
        default=".",
        help="Repository root to initialize. Defaults to the current directory.",
    )
    parser.add_argument(
        "--vcs",
        choices=("git", "graphite", "sapling", "custom"),
        required=True,
        help="VCS preset to install into bin/.",
    )
    parser.add_argument(
        "--sapling-bookmark",
        default="main",
        help="Remote bookmark for the sapling push preset. Defaults to main.",
    )
    parser.add_argument("--custom-pull", help="Shell snippet for bin/pull.")
    parser.add_argument("--custom-push", help="Shell snippet for bin/push.")
    parser.add_argument("--custom-commit", help="Shell snippet for bin/commit.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(args.path).resolve()
    ensure_empty_project(root)
    ensure_targets_do_not_exist(root)

    root.mkdir(parents=True, exist_ok=True)
    (root / "bin").mkdir()

    write_text(root / "AGENTS.md", AGENTS_CONTENT)
    write_text(root / "CLAUDE.md", CLAUDE_CONTENT)
    write_text(root / "TODO.md", TODO_CONTENT)
    write_text(root / "DONE.md", DONE_CONTENT)
    write_text(root / "T_max.md", T_MAX_CONTENT)

    commands = preset_commands(args)
    write_script(root / "bin" / "pull", render_pull_script(commands["pull"]))
    write_script(root / "bin" / "push", render_push_script(commands["push"]))
    write_script(root / "bin" / "commit", render_commit_script(commands["commit"]))

    print(f"Initialized scode-todo project in {root}")
    print(f"Installed VCS preset: {args.vcs}")
    return 0


def ensure_empty_project(root: Path) -> None:
    if root.exists():
        entries = sorted(
            entry.name for entry in root.iterdir() if entry.name not in ALLOWED_EXISTING_ENTRIES
        )
        if entries:
            joined = ", ".join(entries)
            raise SystemExit(
                f"error: {root} is not empty; found non-VCS entries: {joined}"
            )
        return

    root.mkdir(parents=True, exist_ok=True)


def ensure_targets_do_not_exist(root: Path) -> None:
    targets = [
        root / "AGENTS.md",
        root / "CLAUDE.md",
        root / "TODO.md",
        root / "DONE.md",
        root / "T_max.md",
        root / "bin",
    ]

    existing = [str(target.relative_to(root)) for target in targets if target.exists()]
    if existing:
        joined = ", ".join(existing)
        raise SystemExit(f"error: refusing to overwrite existing paths: {joined}")


def preset_commands(args: argparse.Namespace) -> dict[str, str]:
    if args.vcs == "git":
        return {
            "pull": "\n".join(
                [
                    'branch="$(git symbolic-ref --quiet --short HEAD)" || {',
                    '  echo "error: not on a branch" >&2',
                    "  exit 1",
                    "}",
                    "",
                    'git fetch origin "$branch"',
                    'git merge --ff-only "origin/$branch"',
                ]
            ),
            "push": "\n".join(
                [
                    'branch="$(git symbolic-ref --quiet --short HEAD)" || {',
                    '  echo "error: not on a branch" >&2',
                    "  exit 1",
                    "}",
                    "",
                    'git push origin "$branch"',
                ]
            ),
            "commit": "\n".join(
                [
                    "git add -A",
                    'git commit --message="$message"',
                ]
            ),
        }

    if args.vcs == "graphite":
        return {
            "pull": "gt sync --all --force",
            "push": "gt submit --publish --no-edit",
            "commit": 'gt modify --all --commit --message "$message"',
        }

    if args.vcs == "sapling":
        bookmark = args.sapling_bookmark.strip()
        if not bookmark:
            raise SystemExit("error: --sapling-bookmark must be non-empty")

        return {
            "pull": "sl pull --rebase",
            "push": f"sl push --rev . --to {shlex.quote(bookmark)}",
            "commit": 'sl commit --addremove --message "$message"',
        }

    if args.vcs == "custom":
        custom_pull = require_non_empty("custom pull", args.custom_pull)
        custom_push = require_non_empty("custom push", args.custom_push)
        custom_commit = require_non_empty("custom commit", args.custom_commit)

        if "$message" not in custom_commit and "${message}" not in custom_commit:
            raise SystemExit(
                "error: --custom-commit must reference $message or ${message}"
            )

        return {
            "pull": custom_pull,
            "push": custom_push,
            "commit": custom_commit,
        }

    raise AssertionError(f"unhandled vcs preset: {args.vcs}")


def require_non_empty(label: str, value: str | None) -> str:
    if value is None or not value.strip():
        raise SystemExit(f"error: missing {label} command")
    return value


def render_pull_script(command: str) -> str:
    return f"""#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 0 ]; then
  echo "usage: pull" >&2
  exit 2
fi

{command}
"""


def render_push_script(command: str) -> str:
    return f"""#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 0 ]; then
  echo "usage: push" >&2
  exit 2
fi

{command}
"""


def render_commit_script(command: str) -> str:
    return f"""#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: commit \\"message\\"" >&2
  exit 2
fi

message="$1"
if [ -z "$message" ]; then
  echo "error: commit message must be non-empty" >&2
  exit 2
fi

{command}
"""


def write_text(path: Path, content: str) -> None:
    path.write_text(content)


def write_script(path: Path, content: str) -> None:
    path.write_text(content)
    path.chmod(0o755)


if __name__ == "__main__":
    sys.exit(main())
