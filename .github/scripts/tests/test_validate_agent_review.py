from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import validate_agent_review as validator


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
        ):
            with self.subTest(value=value):
                self.assertFalse(validator.nonempty(value))

    def test_nonempty_accepts_completed_values(self) -> None:
        self.assertTrue(validator.nonempty("Reviewed the exact PR commit."))


if __name__ == "__main__":
    unittest.main()
