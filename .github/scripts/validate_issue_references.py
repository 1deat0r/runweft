#!/usr/bin/env python3
"""Extract standalone issue references outside Markdown code from a PR body."""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path


REFERENCE_LINE_RE = re.compile(
    r"^ {0,3}(?:[-*][ \t]+)?(?:closes|fixes|resolves|references)[ \t]+#([0-9]+)(?:[^0-9]|$)",
    re.IGNORECASE,
)
NUMBERED_REFERENCE_RE = re.compile(
    r"(?i)\b(?:closes|fixes|resolves|references)[ \t]+#[0-9]+(?:[^0-9]|$)"
)
HTML_BLOCK_MARKER_RE = re.compile(
    r"(?:<!--|<\?|<![A-Za-z]|<!\[CDATA\[|</?[A-Za-z][A-Za-z0-9-]*(?:[ \t/>]|$))",
    re.IGNORECASE,
)


def extract_issue_numbers(body: str) -> list[int]:
    numbers: set[int] = set()
    inline_ticks: int | None = None
    for line in body.splitlines():
        if inline_ticks is None and (line.startswith("    ") or line.startswith("\t")):
            continue

        visible: list[str] = []
        index = 0
        while index < len(line):
            character = line[index]
            if inline_ticks is None and HTML_BLOCK_MARKER_RE.match(line, index):
                return sorted(numbers)
            if character == "`":
                end = index + 1
                while end < len(line) and line[end] == "`":
                    end += 1
                run_length = end - index
                if inline_ticks is not None:
                    if run_length == inline_ticks:
                        inline_ticks = None
                    index = end
                    continue
                if run_length >= 3:
                    # Fail closed at any fence marker. Requiring the issue
                    # reference before code keeps list and mismatched fences
                    # from being mistaken for a rendered link.
                    return sorted(numbers)
                inline_ticks = run_length
                index = end
                continue

            if character == "~":
                end = index + 1
                while end < len(line) and line[end] == "~":
                    end += 1
                if inline_ticks is None and end - index >= 3:
                    return sorted(numbers)
                if inline_ticks is None:
                    visible.extend(line[index:end])
                index = end
                continue

            if inline_ticks is None:
                visible.append(character)
            index += 1

        reference = REFERENCE_LINE_RE.match("".join(visible))
        if reference:
            numbers.add(int(reference.group(1)))

    return sorted(numbers)


def template_has_numbered_issue_example(template: str) -> bool:
    return NUMBERED_REFERENCE_RE.search(template) is not None


def main() -> None:
    repository_root = Path(__file__).resolve().parents[2]
    template_path = repository_root / ".github" / "PULL_REQUEST_TEMPLATE.md"
    template = template_path.read_text(encoding="utf-8")
    if template_has_numbered_issue_example(template):
        print(
            "The pull request template must not contain a number-shaped example issue reference.",
            file=sys.stderr,
        )
        raise SystemExit(1)

    issue_numbers = extract_issue_numbers(os.environ.get("PR_BODY", ""))
    if not issue_numbers:
        print(
            "Every human-authored pull request must include a standalone Closes, Fixes, Resolves, or References #N line outside code.",
            file=sys.stderr,
        )
        raise SystemExit(1)

    print("\n".join(str(number) for number in issue_numbers))


if __name__ == "__main__":
    main()
