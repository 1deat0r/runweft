from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import validate_issue_references as issue_references


class IssueReferenceParsingTest(unittest.TestCase):
    def test_accepts_a_standalone_first_nonblank_issue_reference(self) -> None:
        body = "\n\nCloses #12\n\nReferences #34"
        self.assertEqual(issue_references.extract_issue_numbers(body), [12])

    def test_rejects_narrative_before_reference(self) -> None:
        self.assertEqual(
            issue_references.extract_issue_numbers("This PR fixes a bug.\nCloses #12"),
            [],
        )

    def test_rejects_issue_references_hidden_by_markdown_or_html(self) -> None:
        bodies = (
            "- Closes #12",
            "`Closes #12`",
            "```text\nCloses #12\n```",
            "~~~text\nCloses #12\n~~~",
            "<!-- Closes #12 -->",
            "<pre>Closes #12</pre>",
            "Example `unclosed\nCloses #12",
        )
        for body in bodies:
            with self.subTest(body=body):
                self.assertEqual(issue_references.extract_issue_numbers(body), [])

    def test_requires_the_whole_line_to_be_a_reference(self) -> None:
        for body in ("Closes #12.\n", "Closes #12 explanation", "Closes #0"):
            with self.subTest(body=body):
                self.assertEqual(issue_references.extract_issue_numbers(body), [])

    def test_template_example_guard_rejects_numeric_example(self) -> None:
        self.assertTrue(
            issue_references.template_has_numbered_issue_example("References #123")
        )
        self.assertFalse(
            issue_references.template_has_numbered_issue_example("References #<N>")
        )


if __name__ == "__main__":
    unittest.main()
