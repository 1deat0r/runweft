#!/usr/bin/env python3
"""Require a standalone GitHub issue reference as the first PR-body line."""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path


REFERENCE_LINE_RE = re.compile(
    r"^ {0,3}(?:closes|fixes|resolves|references)[ \t]+#([1-9][0-9]*)[ \t]*$",
    re.IGNORECASE,
)
NUMBERED_REFERENCE_RE = re.compile(
    r"(?i)\b(?:closes|fixes|resolves|references)[ \t]+#[1-9][0-9]*\b"
)


def extract_issue_numbers(body: str) -> list[int]:
    """Return the issue number only when the first nonblank line is a reference."""
    for line in body.splitlines():
        if not line.strip():
            continue
        reference = REFERENCE_LINE_RE.fullmatch(line)
        return [int(reference.group(1))] if reference else []
    return []


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
            "The first nonblank line of every human-authored pull request must be a standalone Closes, Fixes, Resolves, or References #N issue reference.",
            file=sys.stderr,
        )
        raise SystemExit(1)

    print("\n".join(str(number) for number in issue_numbers))


if __name__ == "__main__":
    main()
