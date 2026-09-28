from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import validate_agent_review as validator


BASE_SHA = "0" * 40
HEAD_SHA = "a" * 40


def valid_receipt() -> dict[str, object]:
    seats = sorted(validator.SEATS)
    reviews = [
        {
            "seat": seat,
            "reviewer_agent": f"reviewer-{index}",
            "reviewed_head_sha": HEAD_SHA,
            "verdict": "APPROVE",
            "summary": f"Reviewed the frozen governance diff from the {seat} perspective.",
        }
        for index, seat in enumerate(seats, start=1)
    ]
    evaluation_reviewer = next(
        review["reviewer_agent"]
        for review in reviews
        if review["seat"] == "evaluation/confounds"
    )
    return {
        "protocol": "runweft-agent-review/v1",
        "reviewed_head_sha": HEAD_SHA,
        "implementer_agent": "implementer",
        "full_board": True,
        "required_seats": seats,
        "omitted_seats": [],
        "reviews": reviews,
        "evaluation": {
            "reviewer_agent": evaluation_reviewer,
            "result": "PASS",
            "scope": "Governance validator and CI acceptance behavior; no runtime behavior.",
            "evidence": "End-to-end validator cases cover valid and rejected receipts.",
        },
        "findings": [],
    }


def invoke_main(
    receipt: dict[str, object],
    *,
    prefix: str = "",
    paths: set[str] | None = None,
) -> tuple[int, str, str]:
    body = (
        prefix
        + "```agent-review\n"
        + json.dumps(receipt)
        + "\n```\n"
    )
    stdout = io.StringIO()
    stderr = io.StringIO()
    environment = {
        "PR_BASE_SHA": BASE_SHA,
        "PR_HEAD_SHA": HEAD_SHA,
        "PR_BODY": body,
    }
    with (
        mock.patch.dict(os.environ, environment, clear=False),
        mock.patch.object(
            validator,
            "changed_paths",
            return_value=paths or {".github/workflows/ci.yml"},
        ),
        contextlib.redirect_stdout(stdout),
        contextlib.redirect_stderr(stderr),
    ):
        try:
            validator.main()
        except SystemExit as exit_error:
            code = int(exit_error.code or 0)
        else:
            code = 0
    return code, stdout.getvalue(), stderr.getvalue()


class ReceiptValidationHelpersTest(unittest.TestCase):
    def test_parse_receipt_json_preserves_unique_nested_values(self) -> None:
        self.assertEqual(
            validator.parse_receipt_json('{"reviews":[{"seat":"systems"}]}'),
            {"reviews": [{"seat": "systems"}]},
        )

    def test_parse_receipt_json_rejects_duplicate_top_level_keys(self) -> None:
        with self.assertRaises(validator.DuplicateJSONKeyError):
            validator.parse_receipt_json(
                '{"reviewed_head_sha":"abc","reviewed_head_sha":"def"}'
            )

    def test_parse_receipt_json_rejects_duplicate_nested_keys(self) -> None:
        with self.assertRaises(validator.DuplicateJSONKeyError):
            validator.parse_receipt_json('{"reviews":[{"seat":"systems","seat":"product"}]}')

    def test_nonempty_rejects_placeholder_values(self) -> None:
        for value in (
            "REPLACE_WITH_REVIEWER_ID",
            "WHY THIS SEAT IS NOT RELEVANT",
            "TODO: explain the evidence",
            "placeholder summary",
            "N/A",
            "NA",
            "Not applicable.",
        ):
            with self.subTest(value=value):
                self.assertFalse(validator.nonempty(value))

    def test_nonempty_accepts_completed_values(self) -> None:
        self.assertTrue(validator.nonempty("Reviewed the exact PR commit."))
        self.assertTrue(
            validator.nonempty(
                "Not applicable because this governance change alters no runtime behavior."
            )
        )

    def test_finding_ids_are_numbered_and_stable(self) -> None:
        for value in ("SYS-1", "security-review-2"):
            with self.subTest(value=value):
                self.assertTrue(validator.valid_finding_id(value))
        for value in ("SYS", "SYS-one", "1", "TODO-1"):
            with self.subTest(value=value):
                self.assertFalse(validator.valid_finding_id(value))

    def test_html_comment_visibility_detection(self) -> None:
        self.assertTrue(validator.is_inside_html_comment("<!-- hidden -->", 5))
        self.assertFalse(validator.is_inside_html_comment("<!-- done --> visible", 15))


class ReceiptValidationMainTest(unittest.TestCase):
    def test_main_accepts_complete_five_seat_receipt_for_changed_workflow(self) -> None:
        code, stdout, stderr = invoke_main(valid_receipt())
        self.assertEqual(code, 0, stderr)
        self.assertIn("5 independent seats", stdout)

    def test_main_rejects_receipt_with_stale_exact_head_sha(self) -> None:
        receipt = valid_receipt()
        receipt["reviewed_head_sha"] = "b" * 40
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("exact current PR head SHA", stderr)

    def test_main_rejects_stale_sha_on_individual_review(self) -> None:
        receipt = valid_receipt()
        receipt["reviews"][0]["reviewed_head_sha"] = "b" * 40  # type: ignore[index]
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("review for", stderr)

    def test_main_requires_every_full_board_review_seat(self) -> None:
        receipt = valid_receipt()
        receipt["reviews"] = receipt["reviews"][:-1]  # type: ignore[index]
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("missing required reviewer seats", stderr)

    def test_main_requires_full_board_for_workflow_changes(self) -> None:
        receipt = valid_receipt()
        receipt["full_board"] = False
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("requires full_board=true", stderr)

    def test_main_rejects_placeholder_not_applicable_evaluation_reason(self) -> None:
        receipt = valid_receipt()
        evaluation = receipt["evaluation"]
        evaluation["result"] = "NOT_APPLICABLE"  # type: ignore[index]
        evaluation.pop("evidence")  # type: ignore[union-attr]
        evaluation["reason"] = "N/A"  # type: ignore[index]
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("requires nonempty reason", stderr)

    def test_main_accepts_reasoned_not_applicable_evaluation(self) -> None:
        receipt = valid_receipt()
        evaluation = receipt["evaluation"]
        evaluation["result"] = "NOT_APPLICABLE"  # type: ignore[index]
        evaluation.pop("evidence")  # type: ignore[union-attr]
        evaluation["reason"] = "No product runtime changed, so behavioral evaluation is not applicable."  # type: ignore[index]
        code, stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 0, stderr)
        self.assertIn("evaluation NOT_APPLICABLE", stdout)

    def test_main_accepts_closed_blocker_bound_to_final_head(self) -> None:
        receipt = valid_receipt()
        receipt["findings"] = [
            {
                "id": "SYS-1",
                "severity": "blocker",
                "location": "validator.py:100",
                "impact": "An unclosed blocker could be merged.",
                "closure_condition": "Close the blocker on this exact head.",
                "status": "closed",
                "closure": {
                    "commit": HEAD_SHA,
                    "quote": "if status != closed:",
                    "evidence": "The current validator checks closure status.",
                },
            }
        ]
        code, stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 0, stderr)
        self.assertIn("1 recorded findings", stdout)

    def test_main_rejects_open_or_misbound_blocker_closure(self) -> None:
        receipt = valid_receipt()
        receipt["findings"] = [
            {
                "id": "SYS-1",
                "severity": "blocker",
                "location": "validator.py:100",
                "impact": "An unclosed blocker could be merged.",
                "closure_condition": "Close the blocker on this exact head.",
                "status": "open",
                "closure": {
                    "commit": HEAD_SHA,
                    "quote": "if status != closed:",
                    "evidence": "The current validator checks closure status.",
                },
            }
        ]
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("must be closed before merge", stderr)

        receipt["findings"][0]["status"] = "closed"  # type: ignore[index]
        receipt["findings"][0]["closure"]["commit"] = "b" * 40  # type: ignore[index]
        code, _stdout, stderr = invoke_main(receipt)
        self.assertEqual(code, 1)
        self.assertIn("closure must bind to the final head SHA", stderr)

    def test_main_rejects_receipt_hidden_inside_html_comment(self) -> None:
        code, _stdout, stderr = invoke_main(valid_receipt(), prefix="<!-- hidden\n")
        self.assertEqual(code, 1)
        self.assertIn("must be visible outside HTML comments", stderr)


if __name__ == "__main__":
    unittest.main()
