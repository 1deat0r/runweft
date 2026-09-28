from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import validate_issue_references as issue_references


class IssueReferenceParsingTest(unittest.TestCase):
    def test_extracts_standalone_and_list_references(self) -> None:
        body = "Closes #12.\n\n- References #34"
        self.assertEqual(issue_references.extract_issue_numbers(body), [12, 34])

    def test_ignores_short_and_mismatched_fences(self) -> None:
        body = (
            "````markdown\n"
            "```\n"
            "References #12\n"
            "~~~\n"
            "Closes #34\n"
            "````\n"
            "Fixes #56\n"
        )
        self.assertEqual(issue_references.extract_issue_numbers(body), [56])

    def test_ignores_tilde_fenced_references(self) -> None:
        body = "~~~md\nReferences #12\n~~~\nResolves #34"
        self.assertEqual(issue_references.extract_issue_numbers(body), [34])

    def test_ignores_inline_and_indented_code_references(self) -> None:
        body = "References `#12`\n    Closes #34\nReferences #56"
        self.assertEqual(issue_references.extract_issue_numbers(body), [56])

    def test_requires_standalone_reference_line(self) -> None:
        self.assertEqual(
            issue_references.extract_issue_numbers("This PR closes #12"), []
        )

    def test_template_example_guard_rejects_numeric_example(self) -> None:
        self.assertTrue(
            issue_references.template_has_numbered_issue_example("References #123")
        )
        self.assertFalse(
            issue_references.template_has_numbered_issue_example("References #<N>")
        )


if __name__ == "__main__":
    unittest.main()
