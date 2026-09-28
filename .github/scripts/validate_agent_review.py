#!/usr/bin/env python3
"""Validate the structure and commit binding of a Runweft PR review receipt."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys


SEATS = {
    "systems/durability",
    "adversarial/security",
    "implementation/developer-experience",
    "evaluation/confounds",
    "product/scope",
}
BASE_SEATS = {
    "implementation/developer-experience",
    "evaluation/confounds",
}
FULL_BOARD_SEATS = SEATS
SHA_RE = re.compile(r"^[0-9a-f]{40}$", re.IGNORECASE)
RECEIPT_RE = re.compile(
    r"(?ms)^```agent-review[ \t]*\r?\n(.*?)^```[ \t]*$"
)
FULL_BOARD_PATHS = {
    "AGENTS.md",
    "CONTRIBUTING.md",
    ".github/PULL_REQUEST_TEMPLATE.md",
    ".github/branch-protection-main.json",
    ".github/scripts/validate_agent_review.py",
    "docs/evaluation-protocol.md",
    "docs/threat-model.md",
    "spec.md",
}
FULL_BOARD_PREFIXES = (
    ".github/scripts/",
    ".github/workflows/",
    "docs/architecture/",
    "docs/normative/",
    "schemas/",
)
SECURITY_PATHS = FULL_BOARD_PREFIXES + (
    ".github/branch-protection-main.json",
    ".github/scripts/",
    ".github/workflows/",
    "AGENTS.md",
    "CONTRIBUTING.md",
    "docs/threat-model.md",
    "migrations/",
)
SYSTEMS_PATHS = (
    "crates/runweft-core/",
    "crates/runweft-daemon/",
    "crates/runweft-protocol/",
    "crates/runweft-cli/",
    "migrations/",
    "schemas/",
)


def fail(message: str) -> None:
    print(f"agent-review-record: {message}", file=sys.stderr)
    raise SystemExit(1)


def nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def changed_paths(base_sha: str, head_sha: str) -> set[str]:
    result = subprocess.run(
        ["git", "diff", "--name-only", "-z", "--no-renames", base_sha, head_sha],
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        fail("could not inspect the PR diff")
    return {path for path in result.stdout.decode("utf-8").split("\0") if path}


def is_full_board_path(path: str) -> bool:
    return path in FULL_BOARD_PATHS or path.startswith(FULL_BOARD_PREFIXES)


def needs_seat(paths: set[str], targets: tuple[str, ...]) -> bool:
    return any(path.startswith(targets) for path in paths)


def main() -> None:
    base_sha = os.environ.get("PR_BASE_SHA", "").lower()
    head_sha = os.environ.get("PR_HEAD_SHA", "").lower()
    body = os.environ.get("PR_BODY", "")
    if not SHA_RE.fullmatch(base_sha) or not SHA_RE.fullmatch(head_sha):
        fail("GitHub did not provide valid base and head commit SHAs")

    matches = RECEIPT_RE.findall(body)
    if len(matches) != 1:
        fail("PR description must contain exactly one ```agent-review JSON block")
    try:
        receipt = json.loads(matches[0])
    except json.JSONDecodeError as exc:
        fail(f"review receipt is not valid JSON: {exc}")
    if not isinstance(receipt, dict):
        fail("review receipt must be a JSON object")

    if receipt.get("protocol") != "runweft-agent-review/v1":
        fail("unsupported or missing review protocol version")
    reviewed_sha = receipt.get("reviewed_head_sha")
    if not isinstance(reviewed_sha, str) or reviewed_sha.lower() != head_sha:
        fail("receipt must identify the exact current PR head SHA")
    implementer = receipt.get("implementer_agent")
    if not nonempty(implementer):
        fail("implementer_agent must identify the implementing agent")

    required = receipt.get("required_seats")
    if not isinstance(required, list) or any(
        not isinstance(seat, str) or seat not in SEATS for seat in required
    ):
        fail("required_seats must be a list of recognized review seats")
    required_set = set(required)
    if len(required_set) != len(required) or not BASE_SEATS.issubset(required_set):
        fail("required_seats must be unique and include implementation and evaluation")

    reviews = receipt.get("reviews")
    if not isinstance(reviews, list):
        fail("reviews must be a list")
    reviewed_seats: set[str] = set()
    reviewers: set[str] = set()
    implementer_key = implementer.strip().casefold()
    for review in reviews:
        if not isinstance(review, dict):
            fail("each review must be a JSON object")
        seat = review.get("seat")
        reviewer = review.get("reviewer_agent")
        if not isinstance(seat, str) or seat not in SEATS or seat in reviewed_seats:
            fail("review seats must be recognized and appear once")
        if not nonempty(reviewer):
            fail(f"review for {seat} must identify its reviewer agent")
        reviewer_key = reviewer.strip().casefold()
        if reviewer_key == implementer_key or reviewer_key in reviewers:
            fail("reviewers must be distinct agents and cannot be the implementer")
        if str(review.get("reviewed_head_sha", "")).lower() != head_sha:
            fail(f"review for {seat} must bind to the exact current PR head SHA")
        if review.get("verdict") != "APPROVE":
            fail(f"review for {seat} must approve the final commit before merge")
        if not nonempty(review.get("summary")):
            fail(f"review for {seat} must include a concise report summary")
        reviewed_seats.add(seat)
        reviewers.add(reviewer_key)
    if not required_set.issubset(reviewed_seats):
        missing = ", ".join(sorted(required_set - reviewed_seats))
        fail(f"missing required reviewer seats: {missing}")
    if reviewed_seats != required_set:
        fail("required_seats must list every reviewer seat in the receipt")

    evaluation = receipt.get("evaluation")
    if not isinstance(evaluation, dict):
        fail("evaluation must be a JSON object")
    eval_reviewer = evaluation.get("reviewer_agent")
    eval_review = next(
        (review for review in reviews if review.get("seat") == "evaluation/confounds"),
        None,
    )
    if not eval_review or not nonempty(eval_reviewer):
        fail("evaluation reviewer must be recorded")
    if eval_reviewer.strip().casefold() != eval_review["reviewer_agent"].strip().casefold():
        fail("evaluation evidence must come from the evaluation/confounds reviewer")
    eval_result = evaluation.get("result")
    if not isinstance(eval_result, str) or eval_result not in {"PASS", "NOT_APPLICABLE"}:
        fail("evaluation result must be PASS or NOT_APPLICABLE")
    evidence_key = "evidence" if eval_result == "PASS" else "reason"
    if not nonempty(evaluation.get(evidence_key)):
        fail(f"evaluation {eval_result} requires nonempty {evidence_key}")
    if not nonempty(evaluation.get("scope")):
        fail("evaluation must state its scope and potential impact")

    findings = receipt.get("findings")
    if not isinstance(findings, list):
        fail("findings must be a list; use [] when reviewers found no blockers")
    finding_ids: set[str] = set()
    for finding in findings:
        if not isinstance(finding, dict):
            fail("each finding must be a JSON object")
        finding_id = finding.get("id")
        if not nonempty(finding_id) or finding_id in finding_ids:
            fail("finding IDs must be present and unique")
        finding_ids.add(finding_id)
        severity = finding.get("severity")
        if not isinstance(severity, str) or severity not in {"blocker", "major", "minor", "note"}:
            fail(f"finding {finding_id} must have a recognized severity")
        if severity == "blocker":
            for field in ("location", "impact", "closure_condition"):
                if not nonempty(finding.get(field)):
                    fail(f"blocker {finding_id} needs {field}")
            closure = finding.get("closure")
            if not isinstance(closure, dict) or finding.get("status") != "closed":
                fail(f"blocker {finding_id} must be closed before merge")
            if str(closure.get("commit", "")).lower() != head_sha:
                fail(f"blocker {finding_id} closure must bind to the final head SHA")
            if not nonempty(closure.get("quote")) or not nonempty(closure.get("evidence")):
                fail(f"blocker {finding_id} needs a quote-verified closure and evidence")

    paths = changed_paths(base_sha, head_sha)
    full_board = receipt.get("full_board")
    if not isinstance(full_board, bool):
        fail("full_board must be true or false")
    if any(is_full_board_path(path) for path in paths) and not full_board:
        fail("this change requires full_board=true")
    if full_board and not FULL_BOARD_SEATS.issubset(required_set):
        fail("full_board=true requires all five seats in required_seats")
    if needs_seat(paths, SECURITY_PATHS) and "adversarial/security" not in required_set:
        fail("workflow, policy, and security-sensitive changes require adversarial/security review")
    if needs_seat(paths, SYSTEMS_PATHS) and "systems/durability" not in required_set:
        fail("core, protocol, schema, and migration changes require systems/durability review")

    print(
        "agent-review-record: valid receipt for "
        f"{head_sha}; {len(reviewed_seats)} independent seats; "
        f"evaluation {eval_result}; {len(findings)} recorded findings"
    )


if __name__ == "__main__":
    main()
