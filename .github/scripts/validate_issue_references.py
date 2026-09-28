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
FENCE_OPEN_RE = re.compile(r"^ {0,3}(`{3,}|~{3,})")
INLINE_CODE_SPAN_RE = re.compile(r"(`+).*?\1")
NUMBERED_REFERENCE_RE = re.compile(
    r"(?i)\b(?:closes|fixes|resolves|references)[ \t]+#[0-9]+(?:[^0-9]|$)"
)


def _closes_fence(line: str, fence_char: str, fence_length: int) -> bool:
    candidate = re.sub(r"^ {0,3}", "", line)
    marker_length = 0
    while candidate[marker_length : marker_length + 1] == fence_char:
        marker_length += 1
    return (
        marker_length >= fence_length
        and not candidate[marker_length:].strip()
    )


def extract_issue_numbers(body: str) -> list[int]:
    numbers: set[int] = set()
    fence_char: str | None = None
    fence_length = 0

    for line in body.splitlines():
        if fence_char is not None:
            if _closes_fence(line, fence_char, fence_length):
                fence_char = None
                fence_length = 0
            continue

        opener = FENCE_OPEN_RE.match(line)
        if opener:
            marker = opener.group(1)
            fence_char = marker[0]
            fence_length = len(marker)
            continue

        if line.startswith("    ") or line.startswith("\t"):
            continue

        without_inline_code = INLINE_CODE_SPAN_RE.sub(" ", line)
        reference = REFERENCE_LINE_RE.match(without_inline_code)
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
